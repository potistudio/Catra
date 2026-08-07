use crate::library::{import_file, LibraryState};
use std::path::PathBuf;
use std::process::Command;
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

    let output = Command::new("yt-dlp")
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
        .output()
        .map_err(|e| format!("yt-dlp を実行できませんでした: {e}"))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(if stderr.trim().is_empty() {
            format!("yt-dlp が終了コード {} で失敗しました", output.status)
        } else {
            stderr.trim().to_string()
        });
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    let file_path = stdout
        .lines()
        .map(str::trim)
        .find(|line| !line.is_empty())
        .ok_or_else(|| "yt-dlp が出力ファイルパスを返しませんでした".to_string())?;

    Ok(PathBuf::from(file_path))
}

pub fn download_and_import(app: AppHandle, url: String) {
    std::thread::spawn(move || {
        let result = (|| {
            let path = download_track(&app, &url)?;
            let state = app.state::<LibraryState>();
            import_file(&state, &path)?;
            Ok::<_, String>(path)
        })();

        match result {
            Ok(path) => {
                let _ = app.emit(
                    "library-updated",
                    path.to_string_lossy().to_string(),
                );
            }
            Err(error) => {
                let _ = app.emit("download-error", error);
            }
        }
    });
}
