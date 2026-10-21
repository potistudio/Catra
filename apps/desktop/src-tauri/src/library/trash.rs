use super::db::LibraryState;
use super::paths::{library_prefix, trash_prefix};
use std::collections::HashSet;
use std::fs;

pub fn trash_tracks(library: &LibraryState, ids: &[i64]) -> Result<u32, String> {
    let targets = collect_with_children(library, ids, false)?;
    if targets.is_empty() {
        return Ok(0);
    }
    let now = chrono::Utc::now().timestamp();
    let mut moved = 0u32;
    for id in targets {
        let Some(track) = library.get_track(id).map_err(|error| error.to_string())? else {
            continue;
        };
        if track.trashed_at.is_some() {
            continue;
        }
        library
            .set_trashed(id, Some(now))
            .map_err(|error| error.to_string())?;
        moved += 1;
    }
    Ok(moved)
}

pub fn restore_tracks(library: &LibraryState, ids: &[i64]) -> Result<u32, String> {
    let targets = collect_with_children(library, ids, true)?;
    if targets.is_empty() {
        return Ok(0);
    }
    let mut restored = 0u32;
    for id in targets {
        let Some(track) = library.get_track(id).map_err(|error| error.to_string())? else {
            continue;
        };
        if track.trashed_at.is_none() {
            continue;
        }
        library
            .set_trashed(id, None)
            .map_err(|error| error.to_string())?;
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

fn collect_ids(library: &LibraryState, id: i64, set: &mut HashSet<i64>) -> Result<(), String> {
    if !set.insert(id) {
        return Ok(());
    }
    for child in library.children_of(id).map_err(|error| error.to_string())? {
        collect_ids(library, child.id, set)?;
    }
    Ok(())
}

fn ensure_rekordbox_writable_if_linked(library: &LibraryState, ids: &[i64]) -> Result<(), String> {
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
    fn trash_and_restore_keep_the_audio_path_and_rekordbox_link() {
        let (library, root) = temp_library();
        let id = insert_file(&library, "song.wav", None);
        let content_id = format!("missing-{}", Uuid::new_v4());
        library
            .set_rekordbox_content_id(id, Some(&content_id))
            .unwrap();
        library
            .set_rekordbox_snapshot(id, Some("saved-cue-data"))
            .unwrap();
        let original = library.get_track(id).unwrap().unwrap();
        assert_eq!(trash_tracks(&library, &[id, id]).unwrap(), 1);
        let trashed = library.get_track(id).unwrap().unwrap();
        assert_eq!(trashed.stored_path, original.stored_path);
        assert_eq!(trashed.path, original.path);
        assert_eq!(
            trashed.rekordbox_content_id.as_deref(),
            Some(content_id.as_str())
        );
        assert_eq!(fs::read(&original.path).unwrap(), b"audio");
        assert_eq!(library.list_trashed().unwrap().len(), 1);
        assert_eq!(trash_tracks(&library, &[id]).unwrap(), 0);
        assert!(trashed.stored_path.ends_with("/song.wav"));
        assert!(trashed.trashed_at.is_some());
        assert_eq!(library.list_tracks().unwrap().len(), 0);
        assert_eq!(restore_tracks(&library, &[id]).unwrap(), 1);
        let restored = library.get_track(id).unwrap().unwrap();
        assert_eq!(restored.stored_path, original.stored_path);
        assert_eq!(restored.path, original.path);
        assert_eq!(
            restored.rekordbox_content_id.as_deref(),
            Some(content_id.as_str())
        );
        assert_eq!(
            library.rekordbox_snapshot(id).unwrap().as_deref(),
            Some("saved-cue-data")
        );
        assert_eq!(fs::read(&original.path).unwrap(), b"audio");
        assert_eq!(restore_tracks(&library, &[id]).unwrap(), 0);
        assert!(restored.trashed_at.is_none());
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn trashing_parent_marks_derivative_without_moving_audio() {
        let (library, root) = temp_library();
        let parent = insert_file(&library, "source.wav", None);
        let child = insert_file(&library, "source.mp3", Some(parent));
        let parent_path = library.get_track(parent).unwrap().unwrap().path;
        let child_path = library.get_track(child).unwrap().unwrap().path;
        assert_eq!(trash_tracks(&library, &[parent]).unwrap(), 2);
        assert_eq!(fs::read(&parent_path).unwrap(), b"audio");
        assert_eq!(fs::read(&child_path).unwrap(), b"audio");
        assert!(library
            .get_track(parent)
            .unwrap()
            .unwrap()
            .trashed_at
            .is_some());
        assert!(library
            .get_track(child)
            .unwrap()
            .unwrap()
            .trashed_at
            .is_some());
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn alt_format_sibling_stays_when_one_is_trashed() {
        let (library, root) = temp_library();
        let first = insert_file(&library, "song.wav", None);
        let second = insert_file(&library, "song.mp3", None);
        library.set_format_group_id(first, Some("group")).unwrap();
        library.set_format_group_id(second, Some("group")).unwrap();
        trash_tracks(&library, &[first]).unwrap();
        assert!(library
            .get_track(first)
            .unwrap()
            .unwrap()
            .trashed_at
            .is_some());
        assert!(library
            .get_track(second)
            .unwrap()
            .unwrap()
            .trashed_at
            .is_none());
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn existing_trash_paths_can_be_restored_without_moving_or_relinking() {
        let (library, root) = temp_library();
        let id = insert_file(&library, "song.wav", None);
        let relative = format!("trash/{id}/song.wav");
        fs::rename(
            library.absolute_path(&library_prefix(id)),
            library.absolute_path(&trash_prefix(id)),
        )
        .unwrap();
        library.update_location(id, &relative).unwrap();
        library.set_trashed(id, Some(1)).unwrap();
        let content_id = format!("missing-{}", Uuid::new_v4());
        library
            .set_rekordbox_content_id(id, Some(&content_id))
            .unwrap();

        assert_eq!(restore_tracks(&library, &[id]).unwrap(), 1);
        let restored = library.get_track(id).unwrap().unwrap();
        assert!(restored.trashed_at.is_none());
        assert_eq!(restored.stored_path, relative);
        assert_eq!(
            restored.rekordbox_content_id.as_deref(),
            Some(content_id.as_str())
        );
        assert_eq!(fs::read(&restored.path).unwrap(), b"audio");
        assert!(!library.absolute_path(&library_prefix(id)).exists());
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn empty_trash_deletes_only_marked_tracks_in_both_storage_locations() {
        let (library, root) = temp_library();
        let logical = insert_file(&library, "logical.wav", None);
        let existing = insert_file(&library, "existing.wav", None);
        let active = insert_file(&library, "active.wav", None);
        trash_tracks(&library, &[logical]).unwrap();
        fs::rename(
            library.absolute_path(&library_prefix(existing)),
            library.absolute_path(&trash_prefix(existing)),
        )
        .unwrap();
        library
            .update_location(existing, &format!("trash/{existing}/existing.wav"))
            .unwrap();
        library.set_trashed(existing, Some(1)).unwrap();

        assert_eq!(empty_trash(&library).unwrap(), 2);
        assert!(library.get_track(logical).unwrap().is_none());
        assert!(library.get_track(existing).unwrap().is_none());
        assert!(!library.absolute_path(&library_prefix(logical)).exists());
        assert!(!library.absolute_path(&trash_prefix(existing)).exists());
        let remaining = library.get_track(active).unwrap().unwrap();
        assert_eq!(fs::read(&remaining.path).unwrap(), b"audio");
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn missing_audio_does_not_block_trash_or_restore() {
        let (library, root) = temp_library();
        let id = insert_file(&library, "missing.wav", None);
        let original = library.get_track(id).unwrap().unwrap();
        fs::remove_file(&original.path).unwrap();
        assert_eq!(trash_tracks(&library, &[id]).unwrap(), 1);
        assert_eq!(restore_tracks(&library, &[id]).unwrap(), 1);
        let restored = library.get_track(id).unwrap().unwrap();
        assert_eq!(restored.stored_path, original.stored_path);
        assert!(restored.trashed_at.is_none());
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn a_database_error_leaves_the_audio_path_and_link_unchanged() {
        let (library, root) = temp_library();
        let id = insert_file(&library, "song.wav", None);
        library
            .set_rekordbox_content_id(id, Some("unchanged-link"))
            .unwrap();
        let original = library.get_track(id).unwrap().unwrap();
        library
            .conn()
            .unwrap()
            .execute_batch(
                "CREATE TRIGGER block_trash BEFORE UPDATE OF trashed_at ON tracks
             BEGIN SELECT RAISE(ABORT, 'blocked'); END;",
            )
            .unwrap();

        assert!(trash_tracks(&library, &[id]).is_err());
        let unchanged = library.get_track(id).unwrap().unwrap();
        assert!(unchanged.trashed_at.is_none());
        assert_eq!(unchanged.stored_path, original.stored_path);
        assert_eq!(
            unchanged.rekordbox_content_id,
            original.rekordbox_content_id
        );
        assert_eq!(fs::read(&unchanged.path).unwrap(), b"audio");
        let _ = fs::remove_dir_all(&root);
    }
}
