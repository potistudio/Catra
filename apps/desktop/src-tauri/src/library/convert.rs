use super::db::{LibraryState, Track};
use super::ingest::ingest_converted;
use super::paths::converted_filename;
use crate::activity_log::emit_activity_log;
use serde::{Deserialize, Serialize};
use std::io::{BufRead, BufReader, Read};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};
use tauri::{AppHandle, Emitter, Manager};

static CONVERTING: AtomicBool = AtomicBool::new(false);

const LOSSY_BITRATES: [u32; 4] = [128, 192, 256, 320];

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ConvertFormat {
    Mp3,
    Aac,
    Flac,
    Wav,
    Aiff,
    Ogg,
    Opus,
}

impl ConvertFormat {
    fn extension(self) -> &'static str {
        match self {
            Self::Mp3 => "mp3",
            Self::Aac => "m4a",
            Self::Flac => "flac",
            Self::Wav => "wav",
            Self::Aiff => "aiff",
            Self::Ogg => "ogg",
            Self::Opus => "opus",
        }
    }

    fn is_lossy(self) -> bool {
        matches!(self, Self::Mp3 | Self::Aac | Self::Ogg | Self::Opus)
    }

    fn supports_cover(self) -> bool {
        matches!(
            self,
            Self::Mp3 | Self::Aac | Self::Flac | Self::Ogg | Self::Opus
        )
    }

    fn default_bitrate(self) -> u32 {
        match self {
            Self::Mp3 => 320,
            Self::Aac => 256,
            Self::Ogg => 192,
            Self::Opus => 128,
            _ => 320,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ConvertOptions {
    pub format: ConvertFormat,
    pub bitrate_kbps: Option<u32>,
    pub bit_depth: Option<u32>,
    pub sample_rate: Option<u32>,
    pub channels: Option<u32>,
    /// When true, each newly converted file is inserted into Rekordbox after library insert.
    #[serde(default)]
    pub add_to_rekordbox: bool,
}

impl ConvertOptions {
    /// Rejects values outside the supported set and fills format-specific defaults.
    pub fn validated(self) -> Result<Self, String> {
        if let Some(rate) = self.sample_rate {
            if rate != 44_100 && rate != 48_000 {
                return Err("サンプルレートは 44100 または 48000 を指定してください".to_string());
            }
        }
        if let Some(channels) = self.channels {
            if channels != 1 && channels != 2 {
                return Err("チャンネルは 1 または 2 を指定してください".to_string());
            }
        }

        if self.format.is_lossy() {
            let bitrate = self
                .bitrate_kbps
                .unwrap_or_else(|| self.format.default_bitrate());
            if !LOSSY_BITRATES.contains(&bitrate) {
                return Err("ビットレートは 128 / 192 / 256 / 320 kbps を指定してください".to_string());
            }
            Ok(Self {
                bitrate_kbps: Some(bitrate),
                bit_depth: None,
                ..self
            })
        } else {
            let bit_depth = self.bit_depth.unwrap_or(16);
            if bit_depth != 16 && bit_depth != 24 {
                return Err("ビット深度は 16 または 24 を指定してください".to_string());
            }
            Ok(Self {
                bitrate_kbps: None,
                bit_depth: Some(bit_depth),
                ..self
            })
        }
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ConvertProgress {
    pub processed: u32,
    pub total: u32,
    pub converted: u32,
    pub skipped: u32,
    pub failed: u32,
    pub current_path: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub percent: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ConvertResult {
    pub converted: u32,
    pub skipped: u32,
    pub failed: u32,
    pub rekordbox_added: u32,
    pub rekordbox_failed: u32,
}

/// Starts sequential conversion on a background thread.
///
/// Does not overwrite source files. Fails the command if another conversion is running.
pub fn start_convert_tracks(
    app: AppHandle,
    ids: Vec<i64>,
    options: ConvertOptions,
) -> Result<(), String> {
    if ids.is_empty() {
        return Err("変換するトラックがありません".to_string());
    }
    if CONVERTING.swap(true, Ordering::SeqCst) {
        return Err("別の変換が実行中です".to_string());
    }

    std::thread::spawn(move || {
        let _guard = ConvertGuard;
        let result = convert_tracks(&app, &ids, options);

        match result {
            Ok(summary) => {
                emit_activity_log(
                    &app,
                    "success",
                    format!(
                        "変換完了: 成功 {} · スキップ {} · 失敗 {}{}",
                        summary.converted,
                        summary.skipped,
                        summary.failed,
                        rekordbox_summary_suffix(&summary)
                    ),
                    None,
                );
                let _ = app.emit("library-convert-complete", &summary);
                let _ = app.emit("library-updated", ());
            }
            Err(error) => {
                emit_activity_log(
                    &app,
                    "error",
                    "変換に失敗しました",
                    Some(error.clone()),
                );
                let _ = app.emit("library-convert-error", error);
            }
        }
    });

    Ok(())
}

struct ConvertGuard;

impl Drop for ConvertGuard {
    fn drop(&mut self) {
        CONVERTING.store(false, Ordering::SeqCst);
    }
}

fn convert_tracks(
    app: &AppHandle,
    ids: &[i64],
    options: ConvertOptions,
) -> Result<ConvertResult, String> {
    let options = options.validated()?;
    ensure_ffmpeg()?;

    let library = app.state::<LibraryState>();
    let total = ids.len() as u32;
    let mut converted = 0u32;
    let mut skipped = 0u32;
    let mut failed = 0u32;
    let mut rekordbox_added = 0u32;
    let mut rekordbox_failed = 0u32;
    let mut add_to_rekordbox = options.add_to_rekordbox;

    if add_to_rekordbox && !rekordbox_accepts_writes(app) {
        add_to_rekordbox = false;
    }

    emit_activity_log(
        app,
        "info",
        format!("{total} 曲を変換します"),
        None,
    );

    for (index, id) in ids.iter().enumerate() {
        let processed = (index + 1) as u32;
        let track = match library.get_track(*id).map_err(|error| error.to_string())? {
            Some(track) => track,
            None => {
                failed += 1;
                emit_activity_log(
                    app,
                    "warning",
                    format!("トラックが見つかりません: {id}"),
                    None,
                );
                emit_convert_progress(
                    app,
                    processed,
                    total,
                    converted,
                    skipped,
                    failed,
                    String::new(),
                    None,
                    Some(format!("トラックが見つかりません ({processed}/{total})")),
                );
                continue;
            }
        };

        let source = PathBuf::from(&track.path);
        let current_name = source
            .file_name()
            .map(|name| name.to_string_lossy().into_owned())
            .unwrap_or_else(|| track.path.clone());

        emit_convert_progress(
            app,
            processed,
            total,
            converted,
            skipped,
            failed,
            track.path.clone(),
            Some(0.0),
            Some(format!("変換中: {current_name} ({processed}/{total})")),
        );

        match convert_one(
            app,
            &library,
            &track,
            &source,
            &options,
            processed,
            total,
            converted,
            skipped,
            failed,
        ) {
            Ok(ConvertOutcome::Converted { output, id }) => {
                converted += 1;
                emit_activity_log(
                    app,
                    "success",
                    format!("変換完了: {current_name}"),
                    None,
                );
                let _ = app.emit("library-updated", ());
                if add_to_rekordbox {
                    match crate::rekordbox::add_content(
                        output.to_string_lossy().into_owned(),
                        track.title.clone(),
                    ) {
                        Ok(content) => {
                            if let Err(error) = library
                                .set_rekordbox_content_id(id, Some(&content.id))
                                .map_err(|error| error.to_string())
                            {
                                rekordbox_failed += 1;
                                emit_activity_log(
                                    app,
                                    "warning",
                                    format!("Rekordbox Content ID の保存に失敗: {current_name}"),
                                    Some(error),
                                );
                            } else {
                                rekordbox_added += 1;
                                emit_activity_log(
                                    app,
                                    "success",
                                    format!("Rekordbox に追加: {current_name}"),
                                    None,
                                );
                            }
                        }
                        Err(error) => {
                            rekordbox_failed += 1;
                            emit_activity_log(
                                app,
                                "warning",
                                format!("Rekordbox への追加に失敗: {current_name}"),
                                Some(error),
                            );
                        }
                    }
                }
            }
            Ok(ConvertOutcome::Skipped) => {
                skipped += 1;
                emit_activity_log(
                    app,
                    "info",
                    format!("スキップ: {current_name}"),
                    None,
                );
            }
            Err(error) => {
                failed += 1;
                emit_activity_log(
                    app,
                    "warning",
                    format!("変換失敗: {current_name}"),
                    Some(error),
                );
            }
        }
    }

    Ok(ConvertResult {
        converted,
        skipped,
        failed,
        rekordbox_added,
        rekordbox_failed,
    })
}

fn rekordbox_summary_suffix(summary: &ConvertResult) -> String {
    if summary.rekordbox_added == 0 && summary.rekordbox_failed == 0 {
        return String::new();
    }
    format!(
        " · Rekordbox 追加 {} · 失敗 {}",
        summary.rekordbox_added, summary.rekordbox_failed
    )
}

/// Returns false when Rekordbox cannot be written. Logs the reason once.
fn rekordbox_accepts_writes(app: &AppHandle) -> bool {
    match crate::rekordbox::check() {
        Ok(status) if status.db_path.is_none() => {
            emit_activity_log(
                app,
                "warning",
                "Rekordbox ライブラリが見つからないため、変換のみ実行します",
                None,
            );
            false
        }
        Ok(status) if status.rekordbox_running => {
            emit_activity_log(
                app,
                "warning",
                "Rekordbox が起動中のため、変換のみ実行します",
                None,
            );
            false
        }
        Ok(_) => true,
        Err(error) => {
            emit_activity_log(
                app,
                "warning",
                "Rekordbox に追加できないため、変換のみ実行します",
                Some(error),
            );
            false
        }
    }
}

enum ConvertOutcome {
    Converted { output: PathBuf, id: i64 },
    Skipped,
}

#[allow(clippy::too_many_arguments)]
fn convert_one(
    app: &AppHandle,
    library: &LibraryState,
    track: &Track,
    source: &Path,
    options: &ConvertOptions,
    processed: u32,
    total: u32,
    converted: u32,
    skipped: u32,
    failed: u32,
) -> Result<ConvertOutcome, String> {
    if !source.is_file() {
        return Err(format!("ファイルがありません: {}", source.display()));
    }

    let incoming_dir = library
        .incoming_dir()
        .join(uuid::Uuid::new_v4().to_string());
    std::fs::create_dir_all(&incoming_dir).map_err(|error| error.to_string())?;
    let filename = converted_filename(source, options.format.extension());
    let output = incoming_dir.join(&filename);

    if let Err(error) = run_ffmpeg(
        app,
        source,
        &output,
        options,
        track.duration_ms,
        processed,
        total,
        converted,
        skipped,
        failed,
    ) {
        let _ = std::fs::remove_dir_all(&incoming_dir);
        return Err(error);
    }

    match ingest_converted(
        library,
        track,
        &output,
        chrono::Utc::now().timestamp(),
    ) {
        Ok(inserted) => {
            let _ = std::fs::remove_dir_all(&incoming_dir);
            Ok(ConvertOutcome::Converted {
                output: PathBuf::from(&inserted.path),
                id: inserted.id,
            })
        }
        Err(error) if error.contains("既にライブラリ") => {
            let _ = std::fs::remove_dir_all(&incoming_dir);
            Ok(ConvertOutcome::Skipped)
        }
        Err(error) => {
            let _ = std::fs::remove_dir_all(&incoming_dir);
            Err(error)
        }
    }
}

fn ensure_ffmpeg() -> Result<(), String> {
    let mut command = Command::new("ffmpeg");
    command
        .arg("-version")
        .stdout(Stdio::null())
        .stderr(Stdio::null());
    configure_ffmpeg_command(&mut command);

    match command.status() {
        Ok(status) if status.success() => Ok(()),
        _ => Err("ffmpeg が見つかりません。PATH に追加してください。".to_string()),
    }
}

#[allow(clippy::too_many_arguments)]
fn run_ffmpeg(
    app: &AppHandle,
    source: &Path,
    output: &Path,
    options: &ConvertOptions,
    duration_ms: Option<u64>,
    processed: u32,
    total: u32,
    converted: u32,
    skipped: u32,
    failed: u32,
) -> Result<(), String> {
    let args = ffmpeg_args(source, output, options);
    let mut command = Command::new("ffmpeg");
    command
        .args(&args)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    configure_ffmpeg_command(&mut command);

    let mut child = command
        .spawn()
        .map_err(|error| format!("ffmpeg を実行できませんでした: {error}"))?;

    let current_path = source.to_string_lossy().into_owned();
    if let Some(stdout) = child.stdout.take() {
        let app = app.clone();
        let current_path = current_path.clone();
        std::thread::spawn(move || {
            read_ffmpeg_progress(
                &app,
                stdout,
                duration_ms,
                processed,
                total,
                converted,
                skipped,
                failed,
                current_path,
            );
        });
    }

    let stderr_thread = child.stderr.take().map(|stderr| {
        std::thread::spawn(move || {
            let mut buf = String::new();
            let _ = BufReader::new(stderr).read_to_string(&mut buf);
            buf
        })
    });

    let status = child
        .wait()
        .map_err(|error| format!("ffmpeg の完了待ちに失敗しました: {error}"))?;
    let stderr = stderr_thread
        .and_then(|thread| thread.join().ok())
        .unwrap_or_default();

    if !status.success() {
        let _ = std::fs::remove_file(output);
        let detail = stderr
            .lines()
            .rev()
            .find(|line| !line.trim().is_empty())
            .unwrap_or("不明なエラー");
        return Err(format!("ffmpeg が失敗しました: {detail}"));
    }

    if !output.is_file() {
        return Err("変換ファイルが作成されませんでした".to_string());
    }

    Ok(())
}

#[allow(clippy::too_many_arguments)]
fn read_ffmpeg_progress(
    app: &AppHandle,
    stdout: impl Read,
    duration_ms: Option<u64>,
    processed: u32,
    total: u32,
    converted: u32,
    skipped: u32,
    failed: u32,
    current_path: String,
) {
    let reader = BufReader::new(stdout);
    let mut last_emit = Instant::now()
        .checked_sub(Duration::from_millis(200))
        .unwrap_or_else(Instant::now);

    for line in reader.lines().map_while(Result::ok) {
        let Some(out_ms) = parse_out_time_ms(&line) else {
            continue;
        };
        if last_emit.elapsed() < Duration::from_millis(200) {
            continue;
        }
        last_emit = Instant::now();

        let percent = duration_ms.filter(|&duration| duration > 0).map(|duration| {
            (out_ms as f64 / duration as f64 * 100.0).clamp(0.0, 100.0)
        });
        let _ = app.emit(
            "library-convert-progress",
            ConvertProgress {
                processed,
                total,
                converted,
                skipped,
                failed,
                current_path: current_path.clone(),
                percent,
                message: None,
            },
        );
    }
}

#[allow(clippy::too_many_arguments)]
fn emit_convert_progress(
    app: &AppHandle,
    processed: u32,
    total: u32,
    converted: u32,
    skipped: u32,
    failed: u32,
    current_path: String,
    percent: Option<f64>,
    message: Option<String>,
) {
    let _ = app.emit(
        "library-convert-progress",
        ConvertProgress {
            processed,
            total,
            converted,
            skipped,
            failed,
            current_path,
            percent,
            message,
        },
    );
}

fn configure_ffmpeg_command(command: &mut Command) {
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        const CREATE_NO_WINDOW: u32 = 0x0800_0000;
        command.creation_flags(CREATE_NO_WINDOW);
    }
}

/// Builds ffmpeg argv. Does not include `-y`. Output must differ from the source path.
pub(crate) fn ffmpeg_args(source: &Path, output: &Path, options: &ConvertOptions) -> Vec<String> {
    let mut args = vec![
        "-n".to_string(),
        "-hide_banner".to_string(),
        "-i".to_string(),
        source.to_string_lossy().into_owned(),
        "-map".to_string(),
        "0:a".to_string(),
        "-map_metadata".to_string(),
        "0".to_string(),
    ];

    if options.format.supports_cover() {
        args.push("-map".to_string());
        args.push("0:v?".to_string());
        args.push("-c:v".to_string());
        args.push("copy".to_string());
    }

    match options.format {
        ConvertFormat::Mp3 => {
            args.extend([
                "-c:a".to_string(),
                "libmp3lame".to_string(),
                "-b:a".to_string(),
                format!("{}k", options.bitrate_kbps.unwrap_or(320)),
                "-id3v2_version".to_string(),
                "3".to_string(),
            ]);
        }
        ConvertFormat::Aac => {
            args.extend([
                "-c:a".to_string(),
                "aac".to_string(),
                "-b:a".to_string(),
                format!("{}k", options.bitrate_kbps.unwrap_or(256)),
            ]);
        }
        ConvertFormat::Flac => {
            args.extend(["-c:a".to_string(), "flac".to_string()]);
            if options.bit_depth.unwrap_or(16) == 24 {
                args.extend(["-sample_fmt".to_string(), "s32".to_string()]);
            } else {
                args.extend(["-sample_fmt".to_string(), "s16".to_string()]);
            }
        }
        ConvertFormat::Wav => {
            let codec = if options.bit_depth.unwrap_or(16) == 24 {
                "pcm_s24le"
            } else {
                "pcm_s16le"
            };
            args.extend(["-c:a".to_string(), codec.to_string()]);
        }
        ConvertFormat::Aiff => {
            let codec = if options.bit_depth.unwrap_or(16) == 24 {
                "pcm_s24be"
            } else {
                "pcm_s16be"
            };
            args.extend(["-c:a".to_string(), codec.to_string()]);
        }
        ConvertFormat::Ogg => {
            args.extend([
                "-c:a".to_string(),
                "libvorbis".to_string(),
                "-b:a".to_string(),
                format!("{}k", options.bitrate_kbps.unwrap_or(192)),
            ]);
        }
        ConvertFormat::Opus => {
            args.extend([
                "-c:a".to_string(),
                "libopus".to_string(),
                "-b:a".to_string(),
                format!("{}k", options.bitrate_kbps.unwrap_or(128)),
            ]);
        }
    }

    if let Some(sample_rate) = options.sample_rate {
        args.extend(["-ar".to_string(), sample_rate.to_string()]);
    }
    if let Some(channels) = options.channels {
        args.extend(["-ac".to_string(), channels.to_string()]);
    }

    args.extend([
        "-progress".to_string(),
        "pipe:1".to_string(),
        "-nostats".to_string(),
        output.to_string_lossy().into_owned(),
    ]);

    args
}

pub(crate) fn parse_out_time_ms(line: &str) -> Option<u64> {
    if let Some(value) = line.strip_prefix("out_time_us=") {
        return value.trim().parse::<u64>().ok().map(|us| us / 1000);
    }
    if let Some(value) = line.strip_prefix("out_time_ms=") {
        return value.trim().parse::<u64>().ok();
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::library::db::{LibraryState, NewTrack};
    use crate::library::paths::{converted_filename, library_prefix};
    use std::fs;

    fn temp_dir() -> PathBuf {
        let dir = std::env::temp_dir().join(format!("catra-convert-{}", uuid::Uuid::new_v4()));
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    fn lossy_options(format: ConvertFormat, bitrate_kbps: u32) -> ConvertOptions {
        ConvertOptions {
            format,
            bitrate_kbps: Some(bitrate_kbps),
            bit_depth: None,
            sample_rate: None,
            channels: None,
            add_to_rekordbox: false,
        }
        .validated()
        .unwrap()
    }

    fn lossless_options(format: ConvertFormat, bit_depth: u32) -> ConvertOptions {
        ConvertOptions {
            format,
            bitrate_kbps: None,
            bit_depth: Some(bit_depth),
            sample_rate: None,
            channels: None,
            add_to_rekordbox: false,
        }
        .validated()
        .unwrap()
    }

    #[test]
    fn converted_output_uses_stem_and_parent_id() {
        let root = temp_dir();
        let library = LibraryState::new(root.clone()).unwrap();
        let incoming = library.incoming_dir().join(uuid::Uuid::new_v4().to_string());
        fs::create_dir_all(&incoming).unwrap();
        let source_file = incoming.join("song title.flac");
        fs::write(&source_file, b"src").unwrap();
        let relative = library.relative_of(&source_file).unwrap();
        let parent_id = library
            .insert_track(NewTrack {
                path: &relative,
                title: Some("Song"),
                artist: None,
                album: None,
                duration_ms: None,
                bpm: None,
                bitrate_kbps: None,
                genre: None,
                key: None,
                rating: None,
                artwork_path: None,
                source: None,
                converted: false,
                added_at: 1,
                content_hash: Some("abc"),
                parent_track_id: None,
                format_group_id: None,
                rekordbox_content_id: None,
            })
            .unwrap();
        let dest = library.absolute_path(&library_prefix(parent_id));
        fs::rename(&incoming, &dest).unwrap();
        library
            .update_location(parent_id, &format!("{}/song title.flac", library_prefix(parent_id)))
            .unwrap();

        let output_dir = library.incoming_dir().join(uuid::Uuid::new_v4().to_string());
        fs::create_dir_all(&output_dir).unwrap();
        let filename = converted_filename(Path::new("song title.flac"), "mp3");
        let output = output_dir.join(&filename);
        fs::write(&output, b"converted-bytes").unwrap();
        let parent = library.get_track(parent_id).unwrap().unwrap();
        let inserted = ingest_converted(&library, &parent, &output, 2).unwrap();
        assert_eq!(inserted.parent_track_id, Some(parent_id));
        assert!(inserted.converted);
        assert!(inserted.stored_path.starts_with("library/"));
        assert!(inserted.stored_path.ends_with("/song title.mp3"));
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn ffmpeg_args_mp3_includes_lame_and_bitrate() {
        let options = lossy_options(ConvertFormat::Mp3, 320);
        let args = ffmpeg_args(Path::new("in.wav"), Path::new("out.mp3"), &options);
        assert!(args.contains(&"-n".to_string()));
        assert!(!args.iter().any(|arg| arg == "-y"));
        assert!(args.contains(&"libmp3lame".to_string()));
        assert!(args.contains(&"320k".to_string()));
        assert_eq!(args.last().map(String::as_str), Some("out.mp3"));
    }

    #[test]
    fn ffmpeg_args_wav_24_uses_pcm_s24le() {
        let options = lossless_options(ConvertFormat::Wav, 24);
        let args = ffmpeg_args(Path::new("in.mp3"), Path::new("out.wav"), &options);
        assert!(args.contains(&"pcm_s24le".to_string()));
        assert!(!args.iter().any(|arg| arg == "-b:a"));
        assert!(!args.contains(&"0:v?".to_string()));
    }

    #[test]
    fn ffmpeg_args_mp3_maps_optional_cover() {
        let options = lossy_options(ConvertFormat::Mp3, 192);
        let args = ffmpeg_args(Path::new("in.flac"), Path::new("out.mp3"), &options);
        assert!(args.contains(&"0:v?".to_string()));
        assert!(args.contains(&"copy".to_string()));
    }

    #[test]
    fn ffmpeg_args_omits_ar_when_keeping_sample_rate() {
        let options = lossy_options(ConvertFormat::Aac, 256);
        let args = ffmpeg_args(Path::new("in.wav"), Path::new("out.m4a"), &options);
        assert!(!args.iter().any(|arg| arg == "-ar"));
        assert!(!args.iter().any(|arg| arg == "-ac"));
        assert!(args.contains(&"aac".to_string()));
    }

    #[test]
    fn ffmpeg_args_includes_sample_rate_and_channels() {
        let options = ConvertOptions {
            format: ConvertFormat::Flac,
            bitrate_kbps: None,
            bit_depth: Some(16),
            sample_rate: Some(44_100),
            channels: Some(2),
            add_to_rekordbox: false,
        }
        .validated()
        .unwrap();
        let args = ffmpeg_args(Path::new("in.mp3"), Path::new("out.flac"), &options);
        assert!(args.contains(&"-ar".to_string()));
        assert!(args.contains(&"44100".to_string()));
        assert!(args.contains(&"-ac".to_string()));
        assert!(args.contains(&"2".to_string()));
        assert!(args.contains(&"s16".to_string()));
    }

    #[test]
    fn parse_out_time_from_us_and_ms() {
        assert_eq!(parse_out_time_ms("out_time_us=2500000"), Some(2500));
        assert_eq!(parse_out_time_ms("out_time_ms=2500"), Some(2500));
        assert_eq!(parse_out_time_ms("progress=continue"), None);
    }

    #[test]
    fn validated_rejects_unknown_bitrate() {
        let options = ConvertOptions {
            format: ConvertFormat::Mp3,
            bitrate_kbps: Some(96),
            bit_depth: None,
            sample_rate: None,
            channels: None,
            add_to_rekordbox: true,
        };
        assert!(options.validated().is_err());
    }

    #[test]
    fn validated_keeps_rekordbox_import_flag() {
        let options = ConvertOptions {
            format: ConvertFormat::Mp3,
            bitrate_kbps: Some(320),
            bit_depth: None,
            sample_rate: None,
            channels: None,
            add_to_rekordbox: true,
        }
        .validated()
        .unwrap();
        assert!(options.add_to_rekordbox);
    }
}
