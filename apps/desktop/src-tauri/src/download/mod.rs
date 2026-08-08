use crate::activity_log::emit_activity_log;
use crate::library::{import_file, DuplicateResolver, LibraryState};
use std::collections::HashSet;
use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::SystemTime;
use tauri::{AppHandle, Emitter, Manager};

pub fn download_dir(app: &AppHandle) -> Result<PathBuf, String> {
    let dir = app
        .path()
        .app_data_dir()
        .map_err(|e| e.to_string())?
        .join("downloads");

    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    Ok(dir)
}

fn list_dir_files(dir: &Path) -> Result<HashSet<PathBuf>, String> {
    let mut files = HashSet::new();
    for entry in std::fs::read_dir(dir).map_err(|e| e.to_string())? {
        let entry = entry.map_err(|e| e.to_string())?;
        let path = entry.path();
        if path.is_file() {
            files.insert(path);
        }
    }
    Ok(files)
}

fn decode_process_output(bytes: &[u8]) -> String {
    if let Ok(text) = std::str::from_utf8(bytes) {
        return text.to_string();
    }

    #[cfg(windows)]
    {
        let (decoded, _, had_errors) = encoding_rs::SHIFT_JIS.decode(bytes);
        if !had_errors {
            return decoded.into_owned();
        }
    }

    String::from_utf8_lossy(bytes).into_owned()
}

fn find_newest_new_file(dir: &Path, before: &HashSet<PathBuf>) -> Result<PathBuf, String> {
    let mut newest: Option<(PathBuf, SystemTime)> = None;

    for entry in std::fs::read_dir(dir).map_err(|e| e.to_string())? {
        let entry = entry.map_err(|e| e.to_string())?;
        let path = entry.path();
        if !path.is_file() || before.contains(&path) {
            continue;
        }

        let modified = entry
            .metadata()
            .map_err(|e| e.to_string())?
            .modified()
            .map_err(|e| e.to_string())?;

        if newest.as_ref().is_none_or(|(_, t)| modified > *t) {
            newest = Some((path, modified));
        }
    }

    newest
        .map(|(path, _)| path)
        .ok_or_else(|| "ダウンロードされたファイルが見つかりませんでした".to_string())
}

fn resolve_downloaded_path(
    output_dir: &Path,
    before: &HashSet<PathBuf>,
    stdout: &[u8],
) -> Result<PathBuf, String> {
    let text = decode_process_output(stdout);
    if let Some(line) = text.lines().map(str::trim).find(|line| !line.is_empty()) {
        let path = PathBuf::from(line);
        if path.is_file() {
            return Ok(path);
        }
    }

    find_newest_new_file(output_dir, before)
}

pub fn download_track(app: &AppHandle, url: &str) -> Result<PathBuf, String> {
    let output_dir = download_dir(app)?;
    let output_template = output_dir.join("%(uploader)s - %(title)s.%(ext)s");
    let files_before = list_dir_files(&output_dir)?;

    emit_activity_log(
        app,
        "info",
        format!("yt-dlp を実行: {url}"),
        Some(format!(
            "出力先: {}",
            output_dir.to_string_lossy()
        )),
    );

    let mut child = Command::new("yt-dlp")
        .env("PYTHONUTF8", "1")
        .env("PYTHONIOENCODING", "utf-8")
        .args([
            "-x",
            "--audio-format",
            "mp3",
            "--embed-thumbnail",
            "--add-metadata",
            "--no-playlist",
            "--encoding",
            "utf-8",
            "-o",
            &output_template.to_string_lossy(),
            "--print",
            "after_move:filepath",
            url,
        ])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| format!("yt-dlp を実行できませんでした: {e}"))?;

    if let Some(stderr) = child.stderr.take() {
        let app_stderr = app.clone();
        std::thread::spawn(move || {
            let reader = BufReader::new(stderr);
            for line in reader.lines().map_while(Result::ok) {
                let trimmed = line.trim();
                if !trimmed.is_empty() {
                    emit_activity_log(&app_stderr, "info", trimmed, None);
                }
            }
        });
    }

    let output = child
        .wait_with_output()
        .map_err(|e| format!("yt-dlp の完了待ちに失敗しました: {e}"))?;

    if !output.status.success() {
        let stderr = decode_process_output(&output.stderr);
        let message = if stderr.trim().is_empty() {
            format!("yt-dlp が終了コード {} で失敗しました", output.status)
        } else {
            stderr.trim().to_string()
        };
        return Err(message);
    }

    let path = resolve_downloaded_path(&output_dir, &files_before, &output.stdout)?;
    emit_activity_log(
        app,
        "success",
        format!("ダウンロード完了: {}", path.file_name().unwrap_or_default().to_string_lossy()),
        Some(path.to_string_lossy().to_string()),
    );

    Ok(path)
}

pub fn download_and_import(app: AppHandle, url: String) {
    std::thread::spawn(move || {
        emit_activity_log(&app, "info", format!("ダウンロードを開始: {url}"), None);

        let result = (|| {
            let path = download_track(&app, &url)?;
            emit_activity_log(&app, "info", "ライブラリへインポート中...", None);
            let state = app.state::<LibraryState>();
            let resolver = app.state::<DuplicateResolver>();
            import_file(&app, &state, &resolver, &path)?;
            Ok::<_, String>(path)
        })();

        match result {
            Ok(path) => {
                emit_activity_log(
                    &app,
                    "success",
                    "ライブラリへ追加しました",
                    Some(path.to_string_lossy().to_string()),
                );
                let _ = app.emit(
                    "library-updated",
                    path.to_string_lossy().to_string(),
                );
            }
            Err(error) => {
                emit_activity_log(&app, "error", "ダウンロードに失敗しました", Some(error.clone()));
                let _ = app.emit("download-error", error);
            }
        }
    });
}
