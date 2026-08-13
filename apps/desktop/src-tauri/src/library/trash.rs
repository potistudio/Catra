use super::db::{LibraryState, Track};
use super::paths::{library_prefix, swap_location_prefix, trash_prefix};
use std::collections::HashSet;
use std::fs;

pub fn trash_tracks(library: &LibraryState, ids: &[i64]) -> Result<u32, String> {
    let targets = collect_with_children(library, ids, false)?;
    if targets.is_empty() {
        return Ok(0);
    }
    ensure_rekordbox_writable_if_linked(library, &targets)?;

    let now = chrono::Utc::now().timestamp();
    let mut moved = 0u32;
    for id in targets {
        let Some(track) = library.get_track(id).map_err(|error| error.to_string())? else {
            continue;
        };
        if track.trashed_at.is_some() {
            continue;
        }
        apply_location_change(library, &track, "library", "trash", Some(now))?;
        moved += 1;
    }
    Ok(moved)
}

pub fn restore_tracks(library: &LibraryState, ids: &[i64]) -> Result<u32, String> {
    let targets = collect_with_children(library, ids, true)?;
    if targets.is_empty() {
        return Ok(0);
    }
    ensure_rekordbox_writable_if_linked(library, &targets)?;

    let mut restored = 0u32;
    for id in targets {
        let Some(track) = library.get_track(id).map_err(|error| error.to_string())? else {
            continue;
        };
        if track.trashed_at.is_none() {
            continue;
        }
        apply_location_change(library, &track, "trash", "library", None)?;
        restored += 1;
    }
    Ok(restored)
}

pub fn delete_permanently(library: &LibraryState, ids: &[i64]) -> Result<u32, String> {
    let mut targets = HashSet::new();
    for id in ids {
        collect_ids(library, *id, &mut targets)?;
    }
    let targets: Vec<i64> = targets.into_iter().collect();
    ensure_rekordbox_writable_if_linked(library, &targets)?;

    let mut deleted = 0u32;
    for id in targets {
        let Some(track) = library.get_track(id).map_err(|error| error.to_string())? else {
            continue;
        };
        if let Some(content_id) = track.rekordbox_content_id.as_deref() {
            delete_rekordbox_content(content_id)?;
        }

        let audio_dir = if track.stored_path.starts_with("trash/") {
            library.absolute_path(&trash_prefix(id))
        } else {
            library.absolute_path(&library_prefix(id))
        };
        if audio_dir.exists() {
            fs::remove_dir_all(&audio_dir).map_err(|error| error.to_string())?;
        }
        if let Some(relative) = &track.stored_artwork_path {
            let artwork = library.absolute_path(relative);
            if artwork.exists() {
                fs::remove_file(&artwork).map_err(|error| error.to_string())?;
            }
        }
        library
            .delete_track_row(id)
            .map_err(|error| error.to_string())?;
        deleted += 1;
    }
    Ok(deleted)
}

pub fn empty_trash(library: &LibraryState) -> Result<u32, String> {
    let ids: Vec<i64> = library
        .list_trashed()
        .map_err(|error| error.to_string())?
        .into_iter()
        .map(|track| track.id)
        .collect();
    delete_permanently(library, &ids)
}

fn collect_with_children(
    library: &LibraryState,
    ids: &[i64],
    trashed_only: bool,
) -> Result<Vec<i64>, String> {
    let mut set = HashSet::new();
    for id in ids {
        let Some(track) = library.get_track(*id).map_err(|error| error.to_string())? else {
            continue;
        };
        if trashed_only && track.trashed_at.is_none() {
            continue;
        }
        collect_ids(library, *id, &mut set)?;
    }
    Ok(set.into_iter().collect())
}

fn collect_ids(
    library: &LibraryState,
    id: i64,
    set: &mut HashSet<i64>,
) -> Result<(), String> {
    if !set.insert(id) {
        return Ok(());
    }
    for child in library.children_of(id).map_err(|error| error.to_string())? {
        collect_ids(library, child.id, set)?;
    }
    Ok(())
}

fn apply_location_change(
    library: &LibraryState,
    track: &Track,
    from_kind: &str,
    to_kind: &str,
    trashed_at: Option<i64>,
) -> Result<(), String> {
    relocate_track_dir(library, track.id, from_kind, to_kind)?;
    let relative = swap_location_prefix(&track.stored_path, from_kind, to_kind);
    if let Err(error) = library.set_trashed(track.id, trashed_at, &relative) {
        let _ = relocate_track_dir(library, track.id, to_kind, from_kind);
        return Err(error.to_string());
    }
    if let Err(error) = relink_folder_path(library, track.id, &relative) {
        revert_location_change(library, track, to_kind, from_kind);
        return Err(error);
    }
    Ok(())
}

fn revert_location_change(library: &LibraryState, track: &Track, from_kind: &str, to_kind: &str) {
    let _ = relocate_track_dir(library, track.id, from_kind, to_kind);
    let _ = library.set_trashed(track.id, track.trashed_at, &track.stored_path);
}

fn relocate_track_dir(
    library: &LibraryState,
    id: i64,
    from_kind: &str,
    to_kind: &str,
) -> Result<(), String> {
    let from = library.absolute_path(&format!("{from_kind}/{id}"));
    let to = library.absolute_path(&format!("{to_kind}/{id}"));
    if !from.exists() {
        return Err(format!("ファイルがありません: {}", from.display()));
    }
    if let Some(parent) = to.parent() {
        fs::create_dir_all(parent).map_err(|error| error.to_string())?;
    }
    fs::rename(&from, &to).map_err(|error| error.to_string())
}

fn relink_folder_path(
    library: &LibraryState,
    id: i64,
    relative: &str,
) -> Result<(), String> {
    let Some(track) = library.get_track(id).map_err(|error| error.to_string())? else {
        return Ok(());
    };
    let Some(content_id) = track.rekordbox_content_id else {
        return Ok(());
    };
    if !rekordbox_configured() {
        return Ok(());
    }
    let absolute = library.absolute_string(relative);
    match crate::rekordbox::update_content_folder_path(&content_id, &absolute) {
        Ok(_) => Ok(()),
        Err(error) if is_missing_content_error(&error) => {
            // Content 行が無いときは Catra 側の移動を継続する。リンク ID を外し、再リンクは利用者が行う。
            library
                .set_rekordbox_content_id(id, None)
                .map_err(|error| error.to_string())?;
            Ok(())
        }
        Err(error) => Err(error),
    }
}

fn is_missing_content_error(error: &str) -> bool {
    error.starts_with("content ") && error.ends_with(" was not found")
}

fn ensure_rekordbox_writable_if_linked(
    library: &LibraryState,
    ids: &[i64],
) -> Result<(), String> {
    let mut linked = false;
    for id in ids {
        if let Some(track) = library.get_track(*id).map_err(|error| error.to_string())? {
            if track.rekordbox_content_id.is_some() {
                linked = true;
                break;
            }
        }
    }
    if !linked || !rekordbox_configured() {
        return Ok(());
    }
    crate::rekordbox::ensure_writable()
}

fn rekordbox_configured() -> bool {
    crate::rekordbox::check()
        .ok()
        .and_then(|status| status.db_path)
        .is_some()
}

fn delete_rekordbox_content(content_id: &str) -> Result<(), String> {
    if !rekordbox_configured() {
        return Ok(());
    }
    match crate::rekordbox::delete_content(content_id.to_string()) {
        Err(error) if error.contains("was not found") => Ok(()),
        other => other,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::library::db::{LibraryState, NewTrack};
    use crate::library::paths::library_prefix;
    use uuid::Uuid;

    fn temp_library() -> (LibraryState, std::path::PathBuf) {
        let root = std::env::temp_dir().join(format!("catra-trash-{}", Uuid::new_v4()));
        fs::create_dir_all(&root).unwrap();
        (LibraryState::new(root.clone()).unwrap(), root)
    }

    fn insert_file(library: &LibraryState, name: &str, parent: Option<i64>) -> i64 {
        let incoming = library.incoming_dir().join(Uuid::new_v4().to_string());
        fs::create_dir_all(&incoming).unwrap();
        let file = incoming.join(name);
        fs::write(&file, b"audio").unwrap();
        let relative = library.relative_of(&file).unwrap();
        let id = library
            .insert_track(NewTrack {
                path: &relative,
                title: Some("Song"),
                artist: Some("Artist"),
                album: None,
                duration_ms: Some(1000),
                bpm: None,
                bitrate_kbps: None,
                genre: None,
                key: None,
                rating: None,
                artwork_path: None,
                source: None,
                converted: parent.is_some(),
                added_at: 1,
                content_hash: Some(name),
                parent_track_id: parent,
                format_group_id: None,
                rekordbox_content_id: None,
            })
            .unwrap();
        let dest = library.absolute_path(&library_prefix(id));
        fs::rename(&incoming, &dest).unwrap();
        let stored = format!("{}/{}", library_prefix(id), name);
        library.update_location(id, &stored).unwrap();
        id
    }

    #[test]
    fn trash_keeps_id_suffix_and_restore_returns() {
        let (library, root) = temp_library();
        let id = insert_file(&library, "song.wav", None);
        trash_tracks(&library, &[id]).unwrap();
        let trashed = library.get_track(id).unwrap().unwrap();
        assert!(trashed.stored_path.starts_with("trash/"));
        assert!(trashed.stored_path.ends_with("/song.wav"));
        assert!(trashed.trashed_at.is_some());
        assert_eq!(library.list_tracks().unwrap().len(), 0);
        restore_tracks(&library, &[id]).unwrap();
        let restored = library.get_track(id).unwrap().unwrap();
        assert!(restored.stored_path.starts_with("library/"));
        assert!(restored.stored_path.ends_with("/song.wav"));
        assert!(restored.trashed_at.is_none());
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn trashing_parent_moves_derivative() {
        let (library, root) = temp_library();
        let parent = insert_file(&library, "source.wav", None);
        let child = insert_file(&library, "source.mp3", Some(parent));
        trash_tracks(&library, &[parent]).unwrap();
        assert!(library.get_track(parent).unwrap().unwrap().trashed_at.is_some());
        assert!(library.get_track(child).unwrap().unwrap().trashed_at.is_some());
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn alt_format_sibling_stays_when_one_is_trashed() {
        let (library, root) = temp_library();
        let first = insert_file(&library, "song.wav", None);
        let second = insert_file(&library, "song.mp3", None);
        library
            .set_format_group_id(first, Some("group"))
            .unwrap();
        library
            .set_format_group_id(second, Some("group"))
            .unwrap();
        trash_tracks(&library, &[first]).unwrap();
        assert!(library.get_track(first).unwrap().unwrap().trashed_at.is_some());
        assert!(library.get_track(second).unwrap().unwrap().trashed_at.is_none());
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn missing_content_error_does_not_match_other_not_found() {
        assert!(is_missing_content_error("content 58253138 was not found"));
        assert!(!is_missing_content_error("Rekordbox master.db was not found"));
        assert!(!is_missing_content_error("file not found: C:\\x"));
        assert!(!is_missing_content_error("playlist 1 was not found"));
    }

    #[test]
    fn revert_location_returns_track_to_trash() {
        let (library, root) = temp_library();
        let id = insert_file(&library, "song.wav", None);
        trash_tracks(&library, &[id]).unwrap();
        let track = library.get_track(id).unwrap().unwrap();
        apply_location_change(&library, &track, "trash", "library", None).unwrap();
        let restored = library.get_track(id).unwrap().unwrap();
        assert!(restored.trashed_at.is_none());
        revert_location_change(&library, &track, "library", "trash");
        let reverted = library.get_track(id).unwrap().unwrap();
        assert!(reverted.trashed_at.is_some());
        assert!(reverted.stored_path.starts_with("trash/"));
        assert!(library.absolute_path(&format!("trash/{id}")).exists());
        assert!(!library.absolute_path(&format!("library/{id}")).exists());
        let _ = fs::remove_dir_all(&root);
    }
}
