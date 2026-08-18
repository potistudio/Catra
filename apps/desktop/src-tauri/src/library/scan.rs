use super::db::{LibraryState, ScanResult};
use super::duplicate::DuplicateResolver;
use super::ingest::{count_outcome, ingest_path, log_ingest_error, IngestOptions};
use crate::activity_log::emit_activity_log;
use lofty::config::ParseOptions;
use lofty::file::AudioFile;
use lofty::file::TaggedFileExt;
use lofty::picture::PictureType;
use lofty::probe::Probe;
use lofty::tag::Accessor;
use lofty::tag::ItemKey;
use serde::Serialize;
use std::path::{Path, PathBuf};
use tauri::{AppHandle, Emitter, Manager};
use walkdir::WalkDir;

const AUDIO_EXTENSIONS: &[&str] = &["mp3", "flac", "wav", "aiff", "aif", "m4a", "ogg", "opus", "wma"];
const SCAN_PROGRESS_INTERVAL: u32 = 10;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ScanProgress {
    pub processed: u32,
    pub added: u32,
    pub skipped: u32,
    pub current_path: String,
    /// Set when the file list is known before processing; absent during folder walks.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub total: Option<u32>,
}

pub fn start_scan_folder(app: AppHandle, folder: String) {
    std::thread::spawn(move || {
        let state = app.state::<LibraryState>();
        let resolver = app.state::<DuplicateResolver>();
        let result = scan_folder(&app, &state, &resolver, &folder);

        match result {
            Ok(scan_result) => {
                emit_activity_log(
                    &app,
                    "success",
                    format!(
                        "スキャン完了: {} 曲を追加 ({} 曲は既存)",
                        scan_result.added,
                        scan_result.skipped
                    ),
                    Some(folder),
                );
                let _ = app.emit("library-scan-complete", scan_result);
                let _ = app.emit("library-updated", ());
            }
            Err(error) => {
                emit_activity_log(&app, "error", "スキャンに失敗しました", Some(error.clone()));
                let _ = app.emit("library-scan-error", error);
            }
        }
    });
}

/// Copies dropped files into the managed library directory.
pub fn start_import_paths(app: AppHandle, paths: Vec<String>) {
    std::thread::spawn(move || {
        let state = app.state::<LibraryState>();
        let resolver = app.state::<DuplicateResolver>();
        let result = import_paths(&app, &state, &resolver, &paths);

        match result {
            Ok(scan_result) => {
                emit_activity_log(
                    &app,
                    "success",
                    format!(
                        "取り込み完了: {} 曲を追加 ({} 曲は既存)",
                        scan_result.added, scan_result.skipped
                    ),
                    None,
                );
                let _ = app.emit("library-scan-complete", scan_result);
                let _ = app.emit("library-updated", ());
            }
            Err(error) => {
                emit_activity_log(
                    &app,
                    "error",
                    "取り込みに失敗しました",
                    Some(error.clone()),
                );
                let _ = app.emit("library-scan-error", error);
            }
        }
    });
}

fn import_paths(
    app: &AppHandle,
    library: &LibraryState,
    resolver: &DuplicateResolver,
    paths: &[String],
) -> Result<ScanResult, String> {
    if paths.is_empty() {
        return Err("インポートするパスがありません".to_string());
    }

    let files = collect_audio_files(paths);
    if files.is_empty() {
        emit_activity_log(
            app,
            "warning",
            "ドロップされた項目に音声ファイルがありません",
            None,
        );
        return Ok(ScanResult {
            added: 0,
            skipped: 0,
        });
    }

    let total = files.len() as u32;
    let added_at = chrono::Utc::now().timestamp();
    let mut added = 0u32;
    let mut skipped = 0u32;
    let mut processed = 0u32;

    for path in files {
        processed += 1;
        let path_str = path.to_string_lossy().to_string();

        if processed == 1 || processed % SCAN_PROGRESS_INTERVAL == 0 || processed == total {
            let progress = ScanProgress {
                processed,
                added,
                skipped,
                current_path: path_str.clone(),
                total: Some(total),
            };
            let _ = app.emit("library-scan-progress", progress);
        }

        let metadata = read_metadata(&path);
        match ingest_path(
            Some(app),
            library,
            Some(resolver),
            &path,
            &metadata,
            added_at,
            IngestOptions::default(),
        ) {
            Ok(outcome) => count_outcome(outcome, &mut added, &mut skipped),
            Err(error) => log_ingest_error(app, &path, error),
        }
    }

    Ok(ScanResult { added, skipped })
}

pub fn scan_folder(
    app: &AppHandle,
    library: &LibraryState,
    resolver: &DuplicateResolver,
    folder: &str,
) -> Result<ScanResult, String> {
    let folder_path = Path::new(folder);
    if !folder_path.is_dir() {
        return Err(format!("Not a directory: {folder}"));
    }

    let added_at = chrono::Utc::now().timestamp();
    let mut added = 0u32;
    let mut skipped = 0u32;
    let mut processed = 0u32;

    for entry in WalkDir::new(folder_path)
        .follow_links(false)
        .into_iter()
        .filter_map(|e| e.ok())
    {
        let path = entry.path();
        if !path.is_file() {
            continue;
        }

        if !is_audio_file(path) {
            continue;
        }

        processed += 1;
        let path_str = path.to_string_lossy().to_string();

        if processed == 1 || processed % SCAN_PROGRESS_INTERVAL == 0 {
            let progress = ScanProgress {
                processed,
                added,
                skipped,
                current_path: path_str.clone(),
                total: None,
            };
            let _ = app.emit("library-scan-progress", progress);
        }

        let metadata = read_metadata(path);
        match ingest_path(
            Some(app),
            library,
            Some(resolver),
            path,
            &metadata,
            added_at,
            IngestOptions::default(),
        ) {
            Ok(outcome) => count_outcome(outcome, &mut added, &mut skipped),
            Err(error) => log_ingest_error(app, path, error),
        }
    }

    Ok(ScanResult { added, skipped })
}

pub fn import_file(
    app: &AppHandle,
    library: &LibraryState,
    resolver: &DuplicateResolver,
    path: &Path,
) -> Result<bool, String> {
    if !path.is_file() {
        return Err(format!("Not a file: {}", path.display()));
    }

    if !is_audio_file(path) {
        return Err(format!("Not an audio file: {}", path.display()));
    }

    let metadata = read_metadata(path);
    let added_at = chrono::Utc::now().timestamp();
    match ingest_path(
        Some(app),
        library,
        Some(resolver),
        path,
        &metadata,
        added_at,
        IngestOptions {
            owned: true,
            ..IngestOptions::default()
        },
    ) {
        Ok(super::ingest::IngestOutcome::Added { .. }) => Ok(true),
        Ok(super::ingest::IngestOutcome::Skipped) => Ok(false),
        Err(error) => Err(error),
    }
}

fn is_audio_file(path: &Path) -> bool {
    path.extension()
        .and_then(|ext| ext.to_str())
        .map(|ext| AUDIO_EXTENSIONS.contains(&ext.to_ascii_lowercase().as_str()))
        .unwrap_or(false)
}

/// Expands dropped files and directories into audio paths. Directories are walked recursively.
pub(crate) fn collect_audio_files(paths: &[String]) -> Vec<PathBuf> {
    let mut files = Vec::new();
    for raw in paths {
        let path = PathBuf::from(raw);
        if path.is_file() {
            if is_audio_file(&path) {
                files.push(path);
            }
            continue;
        }
        if !path.is_dir() {
            continue;
        }
        for entry in WalkDir::new(&path)
            .follow_links(false)
            .into_iter()
            .filter_map(|entry| entry.ok())
        {
            let file = entry.into_path();
            if file.is_file() && is_audio_file(&file) {
                files.push(file);
            }
        }
    }
    files
}

/// Normalize path separators for stable library storage and membership checks.
pub(crate) fn normalize_fs_path(path: &str) -> String {
    let mut normalized = path.replace('/', "\\");
    while normalized.ends_with('\\') && normalized.len() > 3 {
        normalized.pop();
    }
    normalized
}

pub(crate) struct FileMetadata {
    pub title: Option<String>,
    pub artist: Option<String>,
    pub album: Option<String>,
    pub duration_ms: Option<u64>,
    pub bpm: Option<f32>,
    pub bitrate_kbps: Option<u32>,
    pub genre: Option<String>,
    pub key: Option<String>,
    pub rating: Option<u8>,
    pub artwork: Option<(Vec<u8>, String)>,
    /// Absolute artwork file to read after duplicate checks (Rekordbox import).
    pub artwork_file: Option<String>,
    pub source: Option<String>,
}

pub(crate) fn read_metadata(path: &Path) -> FileMetadata {
    let fallback_title = path
        .file_stem()
        .and_then(|s| s.to_str())
        .map(|s| s.to_string());

    let options = ParseOptions::new().max_junk_bytes(4096);
    let tagged_file = match Probe::open(path)
        .and_then(|probe| probe.options(options).read())
    {
        Ok(file) => file,
        Err(_) => {
            return empty_metadata(fallback_title);
        }
    };

    let properties = tagged_file.properties();
    let duration_ms = Some(properties.duration().as_millis() as u64);
    let bitrate_kbps = properties.audio_bitrate();

    if let Some(tag) = tagged_file.primary_tag() {
        let title = tag.title().map(|s| s.to_string()).or(fallback_title);
        let artist = tag.artist().map(|s| s.to_string());
        let album = tag.album().map(|s| s.to_string());
        let genre = tag.genre().map(|s| s.to_string());
        let key = tag
            .get_string(&ItemKey::InitialKey)
            .map(|s| s.to_string());
        let bpm = tag
            .get_string(&ItemKey::Bpm)
            .or_else(|| tag.get_string(&ItemKey::IntegerBpm))
            .and_then(|s| s.parse::<f32>().ok());
        let rating = parse_rating(tag);
        let artwork = extract_artwork(tag);
        let source = detect_source(tag);

        return FileMetadata {
            title,
            artist,
            album,
            duration_ms,
            bpm,
            bitrate_kbps,
            genre,
            key,
            rating,
            artwork,
            artwork_file: None,
            source,
        };
    }

    FileMetadata {
        title: fallback_title,
        artist: None,
        album: None,
        duration_ms,
        bpm: None,
        bitrate_kbps,
        genre: None,
        key: None,
        rating: None,
        artwork: None,
        artwork_file: None,
        source: None,
    }
}

fn empty_metadata(fallback_title: Option<String>) -> FileMetadata {
    FileMetadata {
        title: fallback_title,
        artist: None,
        album: None,
        duration_ms: None,
        bpm: None,
        bitrate_kbps: None,
        genre: None,
        key: None,
        rating: None,
        artwork: None,
        artwork_file: None,
        source: None,
    }
}

fn detect_source(tag: &lofty::tag::Tag) -> Option<String> {
    let url_keys = [
        ItemKey::Comment,
        ItemKey::AudioFileUrl,
        ItemKey::AudioSourceUrl,
        ItemKey::PaymentUrl,
        ItemKey::CommercialInformationUrl,
        ItemKey::TrackArtistUrl,
        ItemKey::Unknown("WWW".to_string()),
        ItemKey::Unknown("www".to_string()),
        ItemKey::Unknown("URL".to_string()),
        ItemKey::Unknown("url".to_string()),
    ];

    for key in url_keys {
        if let Some(value) = tag.get_string(&key) {
            if let Some(source) = detect_source_from_text(value) {
                return Some(source.to_string());
            }
        }
    }

    for item in tag.items() {
        if let Some(text) = item.value().text() {
            if let Some(source) = detect_source_from_text(text) {
                return Some(source.to_string());
            }
        }
    }

    None
}

fn detect_source_from_text(text: &str) -> Option<&'static str> {
    let lower = text.to_ascii_lowercase();
    if lower.contains("soundcloud.com") {
        Some("soundcloud")
    } else if lower.contains("bandcamp.com") {
        Some("bandcamp")
    } else {
        None
    }
}

fn parse_rating(tag: &lofty::tag::Tag) -> Option<u8> {
    if let Some(item) = tag.get(&ItemKey::Popularimeter) {
        if let Some(binary) = item.value().binary() {
            if let Some(null_pos) = binary.iter().position(|&b| b == 0) {
                let rating_index = null_pos + 1;
                if rating_index < binary.len() {
                    let rating = binary[rating_index];
                    if rating > 0 {
                        return Some(rating);
                    }
                }
            }
        }

        if let Some(text) = item.value().text() {
            if let Ok(rating) = text.parse::<u8>() {
                if rating > 0 {
                    return Some(rating);
                }
            }
        }
    }

    None
}

fn extract_artwork(tag: &lofty::tag::Tag) -> Option<(Vec<u8>, String)> {
    let picture = tag
        .get_picture_type(PictureType::CoverFront)
        .or_else(|| tag.pictures().first())?;

    let mime = picture
        .mime_type()
        .map(|m| m.as_str().to_string())
        .unwrap_or_else(|| "image/jpeg".to_string());

    Some((picture.data().to_vec(), mime))
}

pub(crate) fn load_artwork_file(path: Option<&str>) -> Option<(Vec<u8>, String)> {
    let path = path?;
    let data = std::fs::read(path).ok()?;
    if data.is_empty() {
        return None;
    }
    let mime = match Path::new(path)
        .extension()
        .and_then(|ext| ext.to_str())
        .map(|ext| ext.to_ascii_lowercase())
        .as_deref()
    {
        Some("png") => "image/png".to_string(),
        Some("gif") => "image/gif".to_string(),
        Some("webp") => "image/webp".to_string(),
        Some("bmp") => "image/bmp".to_string(),
        _ => "image/jpeg".to_string(),
    };
    Some((data, mime))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    fn temp_dir() -> PathBuf {
        let dir = std::env::temp_dir().join(format!("catra-drop-{}", uuid::Uuid::new_v4()));
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn collect_skips_non_audio_files() {
        let dir = temp_dir();
        let txt = dir.join("notes.txt");
        fs::write(&txt, b"nope").unwrap();
        let files = collect_audio_files(&[txt.to_string_lossy().into_owned()]);
        assert!(files.is_empty());
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn collect_includes_audio_file() {
        let dir = temp_dir();
        let mp3 = dir.join("song.mp3");
        fs::write(&mp3, b"id3").unwrap();
        let files = collect_audio_files(&[mp3.to_string_lossy().into_owned()]);
        assert_eq!(files, vec![mp3]);
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn collect_walks_directory() {
        let dir = temp_dir();
        let nested = dir.join("set");
        fs::create_dir_all(&nested).unwrap();
        let wav = nested.join("track.wav");
        fs::write(&wav, b"riff").unwrap();
        fs::write(nested.join("cover.jpg"), b"img").unwrap();
        let files = collect_audio_files(&[dir.to_string_lossy().into_owned()]);
        assert_eq!(files, vec![wav]);
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn collect_mixes_file_and_directory() {
        let dir = temp_dir();
        let nested = dir.join("set");
        fs::create_dir_all(&nested).unwrap();
        let nested_wav = nested.join("nested.wav");
        let loose_mp3 = dir.join("loose.mp3");
        fs::write(&nested_wav, b"riff").unwrap();
        fs::write(&loose_mp3, b"id3").unwrap();
        let files = collect_audio_files(&[
            nested.to_string_lossy().into_owned(),
            loose_mp3.to_string_lossy().into_owned(),
        ]);
        assert_eq!(files.len(), 2);
        assert!(files.contains(&nested_wav));
        assert!(files.contains(&loose_mp3));
        let _ = fs::remove_dir_all(&dir);
    }
}
