use super::db::LibraryState;
use serde::Serialize;
use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ExportResult {
    pub exported: u32,
    pub skipped: u32,
    pub failed: u32,
    pub errors: Vec<String>,
}

pub fn export_tracks(
    library: &LibraryState,
    ids: &[i64],
    destination: &str,
) -> Result<ExportResult, String> {
    if ids.is_empty() {
        return Err("エクスポートするトラックがありません".to_string());
    }

    let destination = Path::new(destination);
    if !destination.is_dir() {
        return Err(format!(
            "Not a directory: {destination}",
            destination = destination.display()
        ));
    }

    let mut result = ExportResult {
        exported: 0,
        skipped: 0,
        failed: 0,
        errors: Vec::new(),
    };
    let mut seen = HashSet::new();

    for id in ids {
        if !seen.insert(*id) {
            continue;
        }

        let track = match library.get_track(*id).map_err(|error| error.to_string())? {
            Some(track) => track,
            None => {
                record_failure(&mut result, format!("トラックが見つかりません: {id}"));
                continue;
            }
        };

        if track.trashed_at.is_some() {
            record_failure(
                &mut result,
                format!("ゴミ箱のトラックはエクスポートできません: {id}"),
            );
            continue;
        }

        let source = library.absolute_path(&track.stored_path);
        if !source.is_file() {
            record_failure(
                &mut result,
                format!("ファイルが見つかりません: {}", source.display()),
            );
            continue;
        }

        let Some(file_name) = source.file_name() else {
            record_failure(
                &mut result,
                format!("ファイル名を取得できません: {}", source.display()),
            );
            continue;
        };

        let direct_destination = destination.join(file_name);
        if same_path(&source, &direct_destination) {
            result.skipped += 1;
            continue;
        }

        let target = if direct_destination.exists() {
            next_available_path(destination, &source)
        } else {
            direct_destination
        };

        match fs::copy(&source, &target) {
            Ok(_) => result.exported += 1,
            Err(error) => record_failure(&mut result, format!("{}: {}", source.display(), error)),
        }
    }

    Ok(result)
}

fn record_failure(result: &mut ExportResult, error: String) {
    result.failed += 1;
    result.errors.push(error);
}

fn next_available_path(destination: &Path, source: &Path) -> PathBuf {
    let stem = source
        .file_stem()
        .and_then(|value| value.to_str())
        .unwrap_or("track");
    let extension = source
        .extension()
        .and_then(|value| value.to_str())
        .filter(|value| !value.is_empty());

    for suffix in 2.. {
        let name = match extension {
            Some(extension) => format!("{stem} ({suffix}).{extension}"),
            None => format!("{stem} ({suffix})"),
        };
        let candidate = destination.join(name);
        if !candidate.exists() {
            return candidate;
        }
    }

    unreachable!("suffix range is unbounded")
}

fn same_path(left: &Path, right: &Path) -> bool {
    match (fs::canonicalize(left), fs::canonicalize(right)) {
        (Ok(left), Ok(right)) => left == right,
        _ => left == right,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::library::db::{LibraryState, NewTrack};
    use uuid::Uuid;

    fn temp_library() -> (LibraryState, PathBuf) {
        let root = std::env::temp_dir().join(format!("catra-export-{}", Uuid::new_v4()));
        fs::create_dir_all(&root).unwrap();
        let library = LibraryState::new(root.clone()).unwrap();
        (library, root)
    }

    fn insert_track(library: &LibraryState, path: &str) -> i64 {
        library
            .insert_track(NewTrack {
                path,
                title: None,
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
                content_hash: None,
                parent_track_id: None,
                format_group_id: None,
                rekordbox_content_id: None,
            })
            .unwrap()
    }

    #[test]
    fn exports_tracks_without_overwriting_existing_files() {
        let (library, root) = temp_library();
        let source_a = root.join("library/a/song.mp3");
        let source_b = root.join("library/b/song.mp3");
        let destination = root.join("export");
        fs::create_dir_all(source_a.parent().unwrap()).unwrap();
        fs::create_dir_all(source_b.parent().unwrap()).unwrap();
        fs::create_dir_all(&destination).unwrap();
        fs::write(&source_a, b"a").unwrap();
        fs::write(&source_b, b"b").unwrap();

        let first = insert_track(&library, "library/a/song.mp3");
        let second = insert_track(&library, "library/b/song.mp3");
        let result =
            export_tracks(&library, &[first, second], destination.to_str().unwrap()).unwrap();

        assert_eq!(result.exported, 2);
        assert_eq!(result.failed, 0);
        assert_eq!(fs::read(destination.join("song.mp3")).unwrap(), b"a");
        assert_eq!(fs::read(destination.join("song (2).mp3")).unwrap(), b"b");
        let _ = fs::remove_dir_all(root);
    }
}
