use super::db::{LibraryState, ScanResult};
use super::duplicate::DuplicateResolver;
use super::scan::{add_track, FileMetadata, InsertOutcome, ScanProgress};
use crate::activity_log::emit_activity_log;
use crate::rekordbox::{get_content, RekordboxContent};
use std::path::Path;
use tauri::{AppHandle, Emitter, Manager};

const IMPORT_PROGRESS_INTERVAL: u32 = 10;

/// Import Rekordbox collection rows into `library.db` using Rekordbox metadata.
///
/// Skips paths already present in the library and files missing on disk.
/// Emits the same progress/complete events as folder scan.
pub fn start_import_from_rekordbox(app: AppHandle) {
    std::thread::spawn(move || {
        let state = app.state::<LibraryState>();
        let resolver = app.state::<DuplicateResolver>();
        let result = import_from_rekordbox(&app, &state, &resolver);

        match result {
            Ok(scan_result) => {
                emit_activity_log(
                    &app,
                    "success",
                    format!(
                        "Rekordbox から {} 曲を追加 ({} 曲は既存/欠落)",
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
                    "Rekordbox からのインポートに失敗しました",
                    Some(error.clone()),
                );
                let _ = app.emit("library-scan-error", error);
            }
        }
    });
}

fn import_from_rekordbox(
    app: &AppHandle,
    library: &LibraryState,
    resolver: &DuplicateResolver,
) -> Result<ScanResult, String> {
    let contents = get_content(None)?;
    let added_at = chrono::Utc::now().timestamp();
    let mut added = 0u32;
    let mut skipped = 0u32;
    let mut processed = 0u32;

    for content in contents {
        processed += 1;
        let path_str = content.folder_path.clone();

        if processed == 1 || processed % IMPORT_PROGRESS_INTERVAL == 0 {
            let progress = ScanProgress {
                processed,
                added,
                skipped,
                current_path: path_str.clone(),
            };
            let _ = app.emit("library-scan-progress", progress);
        }

        let path = Path::new(&path_str);
        if !path.is_file() {
            skipped += 1;
            emit_activity_log(
                app,
                "warning",
                format!(
                    "スキップ: {}",
                    path.file_name()
                        .unwrap_or_default()
                        .to_string_lossy()
                ),
                Some(format!("ファイルが見つかりません: {path_str}")),
            );
            continue;
        }

        let metadata = metadata_from_rekordbox(&content);

        match add_track(app, library, resolver, path, &metadata, added_at) {
            Ok(InsertOutcome::Added) => added += 1,
            Ok(InsertOutcome::Skipped) => skipped += 1,
            Err(error) => {
                skipped += 1;
                emit_activity_log(
                    app,
                    "warning",
                    format!(
                        "スキップ: {}",
                        path.file_name()
                            .unwrap_or_default()
                            .to_string_lossy()
                    ),
                    Some(error.to_string()),
                );
            }
        }
    }

    Ok(ScanResult { added, skipped })
}

fn metadata_from_rekordbox(content: &RekordboxContent) -> FileMetadata {
    let duration_ms = content
        .length_secs
        .filter(|&secs| secs >= 0)
        .map(|secs| secs as u64 * 1000);
    let bitrate_kbps = content
        .bit_rate
        .filter(|&rate| rate >= 0)
        .map(|rate| rate as u32);
    let rating = content
        .rating
        .map(|value| value.clamp(0, 255) as u8);
    let artwork = content
        .artwork_path
        .as_deref()
        .and_then(load_artwork_file);

    FileMetadata {
        title: content.title.clone(),
        artist: content.artist.clone(),
        album: content.album.clone(),
        duration_ms,
        bpm: content.bpm,
        bitrate_kbps,
        genre: content.genre.clone(),
        key: content.key.clone(),
        rating,
        artwork,
        source: None,
    }
}

fn load_artwork_file(path: &str) -> Option<(Vec<u8>, String)> {
    let data = std::fs::read(path).ok()?;
    if data.is_empty() {
        return None;
    }
    let mime = mime_from_path(Path::new(path));
    Some((data, mime))
}

fn mime_from_path(path: &Path) -> String {
    match path
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
    }
}
