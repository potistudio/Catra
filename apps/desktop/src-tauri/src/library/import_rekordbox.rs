use super::db::{LibraryState, ScanResult};
use super::duplicate::DuplicateResolver;
use super::scan::{
    add_track, normalize_fs_path, FileMetadata, InsertOutcome, ScanProgress,
};
use crate::activity_log::emit_activity_log;
use crate::rekordbox::{get_content, RekordboxContent};
use std::collections::HashSet;
use std::path::{Path, PathBuf};
use tauri::{AppHandle, Emitter, Manager};

const IMPORT_PROGRESS_INTERVAL: u32 = 10;
const MISSING_SAMPLE_LIMIT: usize = 5;

/// Import Rekordbox collection rows into `library.db` using Rekordbox metadata.
///
/// Skips paths already present in the library and files missing on disk.
/// Title/artist/duration collisions use the same interactive duplicate dialog as folder scan.
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
                        "Rekordbox から {} 曲を追加 ({} 曲はスキップ)",
                        scan_result.added, scan_result.skipped
                    ),
                    None,
                );
                let _ = app.emit("library-scan-complete", &scan_result);
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
    let total = contents.len() as u32;
    let added_at = chrono::Utc::now().timestamp();
    let mut added = 0u32;
    let mut skipped = 0u32;
    let mut missing = 0u32;
    let mut already_present = 0u32;
    let mut processed = 0u32;
    let mut missing_samples: Vec<String> = Vec::new();

    let mut existing_keys: HashSet<String> = library
        .list_tracks()
        .map_err(|error| error.to_string())?
        .into_iter()
        .map(|track| path_key(&track.path))
        .collect();

    let _ = app.emit(
        "library-scan-progress",
        &ScanProgress {
            processed: 0,
            added: 0,
            skipped: 0,
            current_path: format!("Rekordbox コレクション {total} 曲を取り込み開始"),
            total: Some(total),
        },
    );

    for content in contents {
        processed += 1;
        let display_path = content.folder_path.clone();

        if processed == 1 || processed % IMPORT_PROGRESS_INTERVAL == 0 || processed == total {
            let progress = ScanProgress {
                processed,
                added,
                skipped,
                current_path: display_path.clone(),
                total: Some(total),
            };
            let _ = app.emit("library-scan-progress", &progress);
        }

        let Some(path) = resolve_content_path(&content) else {
            missing += 1;
            skipped += 1;
            if missing_samples.len() < MISSING_SAMPLE_LIMIT {
                missing_samples.push(display_path);
            }
            continue;
        };

        let normalized = normalize_fs_path(&path.to_string_lossy());
        if existing_keys.contains(&path_key(&normalized)) {
            already_present += 1;
            skipped += 1;
            continue;
        }

        let metadata = metadata_from_rekordbox(&content);

        match add_track(app, library, resolver, &path, &metadata, added_at) {
            Ok(InsertOutcome::Added) => {
                added += 1;
                existing_keys.insert(path_key(&normalized));
            }
            Ok(InsertOutcome::Skipped) => {
                already_present += 1;
                skipped += 1;
            }
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

    if missing > 0 {
        let sample = if missing_samples.is_empty() {
            None
        } else {
            Some(format!(
                "例: {}{}",
                missing_samples.join(" | "),
                if missing as usize > missing_samples.len() {
                    " …"
                } else {
                    ""
                }
            ))
        };
        emit_activity_log(
            app,
            "warning",
            format!("ファイル未検出: {missing} 曲"),
            sample,
        );
    }

    if already_present > 0 {
        emit_activity_log(
            app,
            "info",
            format!("ライブラリ登録済みのためスキップ: {already_present} 曲"),
            None,
        );
    }

    Ok(ScanResult { added, skipped })
}

fn path_key(path: &str) -> String {
    normalize_fs_path(path).to_ascii_lowercase()
}

fn resolve_content_path(content: &RekordboxContent) -> Option<PathBuf> {
    let raw = content.folder_path.trim();
    if raw.is_empty() {
        return None;
    }

    let normalized = PathBuf::from(normalize_fs_path(raw));
    if normalized.is_file() {
        return Some(normalized);
    }

    let original = PathBuf::from(raw);
    if original.is_file() {
        return Some(PathBuf::from(normalize_fs_path(
            &original.to_string_lossy(),
        )));
    }

    // Some rows store the directory in FolderPath and the file name separately.
    if let Some(file_name) = content.file_name.as_deref().filter(|name| !name.is_empty()) {
        for base in [&normalized, &original] {
            if base.is_dir() {
                let joined = base.join(file_name);
                if joined.is_file() {
                    return Some(PathBuf::from(normalize_fs_path(
                        &joined.to_string_lossy(),
                    )));
                }
            }
        }
    }

    None
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
