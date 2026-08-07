use crate::activity_log::emit_activity_log;
use crate::library::{import_file, LibraryState};
use std::io::{BufRead, BufReader};
use std::path::PathBuf;
use std::process::{Command, Stdio};
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

pub fn download_track(app: &AppHandle, url: &str) -> Result<PathBuf, String> {
    let output_dir = download_dir(app)?;
    let output_template = output_dir.join("%(uploader)s - %(title)s.%(ext)s");

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
        .args([
            "-x",
            "--audio-format",
            "mp3",
            "--embed-thumbnail",
            "--add-metadata",
            "--no-playlist",
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
        let stderr = String::from_utf8_lossy(&output.stderr);
        let message = if stderr.trim().is_empty() {
            format!("yt-dlp が終了コード {} で失敗しました", output.status)
        } else {
            stderr.trim().to_string()
        };
        return Err(message);
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    let file_path = stdout
        .lines()
        .map(str::trim)
        .find(|line| !line.is_empty())
        .ok_or_else(|| "yt-dlp が出力ファイルパスを返しませんでした".to_string())?;

    let path = PathBuf::from(file_path);
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
            import_file(&state, &path)?;
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
