use super::db::{save_artwork_for_track, LibraryState, NewTrack, Track};
use super::duplicate::{DuplicateChoice, DuplicateResolver, TrackCandidate};
use super::paths::{
    filename_for_ingest, is_alt_format, is_under, library_prefix, managed_track_id,
};
use super::scan::{load_artwork_file, FileMetadata};
use super::trash;
use crate::activity_log::emit_activity_log;
use std::fs;
use std::io::Read;
use std::path::{Path, PathBuf};
use tauri::AppHandle;
use uuid::Uuid;
use xxhash_rust::xxh64::Xxh64;

const XXHASH_SEED: u64 = 0;

pub enum IngestOutcome {
    Added { id: i64 },
    Skipped,
}

pub struct IngestOptions {
    pub owned: bool,
    pub rekordbox_content_id: Option<String>,
    pub parent_track_id: Option<i64>,
    pub converted: bool,
    pub format_group_id: Option<String>,
    pub skip_duplicate_dialog: bool,
}

impl Default for IngestOptions {
    fn default() -> Self {
        Self {
            owned: false,
            rekordbox_content_id: None,
            parent_track_id: None,
            converted: false,
            format_group_id: None,
            skip_duplicate_dialog: false,
        }
    }
}

pub fn hash_file(path: &Path) -> Result<String, String> {
    let mut file = fs::File::open(path).map_err(|error| error.to_string())?;
    let mut hasher = Xxh64::new(XXHASH_SEED);
    let mut buf = [0u8; 65_536];
    loop {
        let read = file.read(&mut buf).map_err(|error| error.to_string())?;
        if read == 0 {
            break;
        }
        hasher.update(&buf[..read]);
    }
    Ok(format!("{:016x}", hasher.digest()))
}

pub fn ingest_path(
    app: Option<&AppHandle>,
    library: &LibraryState,
    resolver: Option<&DuplicateResolver>,
    path: &Path,
    metadata: &FileMetadata,
    added_at: i64,
    mut options: IngestOptions,
) -> Result<IngestOutcome, String> {
    if !path.is_file() {
        return Err(format!("Not a file: {}", path.display()));
    }

    if let Some(relative) = library.relative_of(path) {
        if let Some(existing) = library
            .find_by_stored_path(&relative)
            .map_err(|error| error.to_string())?
        {
            relink_rekordbox(
                library,
                &existing,
                options.rekordbox_content_id.as_deref(),
            )?;
            return Ok(IngestOutcome::Skipped);
        }
        if let Some(track_id) = managed_track_id(&relative) {
            if library
                .get_track(track_id)
                .map_err(|error| error.to_string())?
                .is_some()
            {
                return Ok(IngestOutcome::Skipped);
            }
        }
    }

    let hash = hash_file(path)?;
    if let Some(existing) = library
        .find_by_content_hash(&hash)
        .map_err(|error| error.to_string())?
    {
        relink_rekordbox(
            library,
            &existing,
            options.rekordbox_content_id.as_deref(),
        )?;
        return Ok(IngestOutcome::Skipped);
    }

    if !options.skip_duplicate_dialog {
        if let Some(existing) = library
            .find_duplicate(
                metadata.title.as_deref(),
                metadata.artist.as_deref(),
                metadata.duration_ms,
            )
            .map_err(|error| error.to_string())?
        {
            let allow_alt_format = is_alt_format(Path::new(&existing.path), path);
            let (app, resolver) = match (app, resolver) {
                (Some(app), Some(resolver)) => (app, resolver),
                _ => {
                    return Err("重複確認が必要ですが UI が接続されていません".to_string());
                }
            };
            let candidate = TrackCandidate {
                path: path.to_string_lossy().into_owned(),
                title: metadata.title.clone(),
                artist: metadata.artist.clone(),
                album: metadata.album.clone(),
                duration_ms: metadata.duration_ms,
                bpm: metadata.bpm,
                bitrate_kbps: metadata.bitrate_kbps,
                genre: metadata.genre.clone(),
                key: metadata.key.clone(),
                rating: metadata.rating,
                source: metadata.source.clone(),
            };
            let existing_id = existing.id;
            let choice = resolver.request_choice(app, existing.clone(), candidate, allow_alt_format);
            match choice {
                DuplicateChoice::KeepExisting => {
                    relink_rekordbox(
                        library,
                        &existing,
                        options.rekordbox_content_id.as_deref(),
                    )?;
                    return Ok(IngestOutcome::Skipped);
                }
                DuplicateChoice::KeepNew => {
                    trash::trash_tracks(library, &[existing_id])?;
                }
                DuplicateChoice::KeepAsAltFormat => {
                    if !allow_alt_format {
                        return Err("同じフォーマットでは別フォーマットとして取り込めません".to_string());
                    }
                    let group_id = existing
                        .format_group_id
                        .clone()
                        .unwrap_or_else(|| Uuid::new_v4().to_string());
                    if existing.format_group_id.is_none() {
                        library
                            .set_format_group_id(existing.id, Some(&group_id))
                            .map_err(|error| error.to_string())?;
                    }
                    options.format_group_id = Some(group_id);
                }
            }
        }
    }

    commit_new_track(library, path, metadata, added_at, &hash, &options)
}

fn relink_rekordbox(
    library: &LibraryState,
    existing: &Track,
    rekordbox_content_id: Option<&str>,
) -> Result<(), String> {
    let Some(content_id) = rekordbox_content_id else {
        return Ok(());
    };
    if !rekordbox_configured() {
        library
            .set_rekordbox_content_id(existing.id, Some(content_id))
            .map_err(|error| error.to_string())?;
        return Ok(());
    }
    crate::rekordbox::update_content_folder_path(content_id, &existing.path)?;
    library
        .set_rekordbox_content_id(existing.id, Some(content_id))
        .map_err(|error| error.to_string())?;
    Ok(())
}

fn rekordbox_configured() -> bool {
    crate::rekordbox::check()
        .ok()
        .and_then(|status| status.db_path)
        .is_some()
}

fn commit_new_track(
    library: &LibraryState,
    source: &Path,
    metadata: &FileMetadata,
    added_at: i64,
    hash: &str,
    options: &IngestOptions,
) -> Result<IngestOutcome, String> {
    let owned = options.owned || is_under(&library.incoming_dir(), source) || {
        is_under(library.root(), source)
            && library.relative_of(source).and_then(|relative| managed_track_id(&relative)).is_none()
    };

    let incoming_dir = library.incoming_dir().join(Uuid::new_v4().to_string());
    fs::create_dir_all(&incoming_dir).map_err(|error| error.to_string())?;
    let staged_name = filename_for_ingest(source, None);
    let staged_path = incoming_dir.join(&staged_name);

    let stage_result = if owned {
        move_or_copy(source, &staged_path)
    } else {
        fs::copy(source, &staged_path)
            .map(|_| ())
            .map_err(|error| error.to_string())
    };
    if let Err(error) = stage_result {
        let _ = fs::remove_dir_all(&incoming_dir);
        return Err(error);
    }

    let incoming_relative = library
        .relative_of(&staged_path)
        .ok_or_else(|| "incoming パスを相対化できません".to_string())?;

    let insert = NewTrack {
        path: &incoming_relative,
        title: metadata.title.as_deref(),
        artist: metadata.artist.as_deref(),
        album: metadata.album.as_deref(),
        duration_ms: metadata.duration_ms,
        bpm: metadata.bpm,
        bitrate_kbps: metadata.bitrate_kbps,
        genre: metadata.genre.as_deref(),
        key: metadata.key.as_deref(),
        rating: metadata.rating,
        artwork_path: None,
        source: metadata.source.as_deref(),
        converted: options.converted,
        added_at,
        content_hash: Some(hash),
        parent_track_id: options.parent_track_id,
        format_group_id: options.format_group_id.as_deref(),
        rekordbox_content_id: None,
    };

    let id = match library.insert_track(insert) {
        Ok(id) => id,
        Err(error) => {
            let _ = fs::remove_dir_all(&incoming_dir);
            return Err(error.to_string());
        }
    };

    let final_name = filename_for_ingest(source, Some(id));
    let library_dir = library.absolute_path(&library_prefix(id));
    if let Err(error) = fs::rename(&incoming_dir, &library_dir) {
        let _ = fs::remove_dir_all(&incoming_dir);
        let _ = library.delete_track_row(id);
        return Err(error.to_string());
    }

    let final_file = library_dir.join(&final_name);
    if staged_name != final_name {
        let staged_final = library_dir.join(&staged_name);
        if let Err(error) = fs::rename(&staged_final, &final_file) {
            let _ = fs::remove_dir_all(&library_dir);
            let _ = library.delete_track_row(id);
            return Err(error.to_string());
        }
    }

    let relative_path = format!("{}/{}", library_prefix(id), final_name);
    if let Err(error) = library.update_location(id, &relative_path) {
        let _ = fs::remove_dir_all(&library_dir);
        let _ = library.delete_track_row(id);
        return Err(error.to_string());
    }

    if let Some((data, mime)) = metadata
        .artwork
        .clone()
        .or_else(|| load_artwork_file(metadata.artwork_file.as_deref()))
    {
        if let Some(artwork_relative) =
            save_artwork_for_track(library.artwork_dir(), id, &data, &mime)
        {
            let _ = library.update_artwork_path(id, Some(&artwork_relative));
        }
    }

    let absolute = library.absolute_string(&relative_path);
    if let Some(content_id) = options.rekordbox_content_id.as_deref() {
        if rekordbox_configured() {
            if let Err(error) = crate::rekordbox::update_content_folder_path(content_id, &absolute) {
                rollback_committed(library, id);
                return Err(error);
            }
        }
        if let Err(error) = library.set_rekordbox_content_id(id, Some(content_id)) {
            rollback_committed(library, id);
            return Err(error.to_string());
        }
    }

    Ok(IngestOutcome::Added { id })
}

fn rollback_committed(library: &LibraryState, id: i64) {
    let dir = library.absolute_path(&library_prefix(id));
    let _ = fs::remove_dir_all(&dir);
    if let Ok(Some(track)) = library.get_track(id) {
        if let Some(relative) = &track.stored_artwork_path {
            let _ = fs::remove_file(library.absolute_path(relative));
        }
    }
    let _ = library.delete_track_row(id);
}

fn move_or_copy(source: &Path, dest: &Path) -> Result<(), String> {
    match fs::rename(source, dest) {
        Ok(()) => Ok(()),
        Err(_) => {
            fs::copy(source, dest).map_err(|error| error.to_string())?;
            fs::remove_file(source).map_err(|error| error.to_string())?;
            Ok(())
        }
    }
}

pub fn ingest_converted(
    library: &LibraryState,
    source_track: &Track,
    output: &Path,
    added_at: i64,
) -> Result<Track, String> {
    let metadata = super::scan::read_metadata(output);
    let options = IngestOptions {
        owned: true,
        rekordbox_content_id: None,
        parent_track_id: Some(source_track.id),
        converted: true,
        format_group_id: None,
        skip_duplicate_dialog: true,
    };
    match ingest_path(None, library, None, output, &metadata, added_at, options)? {
        IngestOutcome::Added { id } => library
            .get_track(id)
            .map_err(|error| error.to_string())?
            .ok_or_else(|| "変換結果を登録できませんでした".to_string()),
        IngestOutcome::Skipped => Err("変換結果は既にライブラリにあります".to_string()),
    }
}

pub fn log_ingest_error(app: &AppHandle, path: &Path, error: String) {
    emit_activity_log(
        app,
        "warning",
        format!(
            "スキップ: {}",
            path.file_name().unwrap_or_default().to_string_lossy()
        ),
        Some(error),
    );
}

pub fn count_outcome(outcome: IngestOutcome, added: &mut u32, skipped: &mut u32) {
    match outcome {
        IngestOutcome::Added { .. } => *added += 1,
        IngestOutcome::Skipped => *skipped += 1,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::library::db::LibraryState;

    fn temp_library() -> (LibraryState, PathBuf) {
        let root = std::env::temp_dir().join(format!("catra-ingest-{}", Uuid::new_v4()));
        fs::create_dir_all(&root).unwrap();
        let library = LibraryState::new(root.clone()).unwrap();
        (library, root)
    }

    fn wav_meta() -> FileMetadata {
        FileMetadata {
            title: Some("Song".to_string()),
            artist: Some("Artist".to_string()),
            album: None,
            duration_ms: Some(1000),
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

    #[test]
    fn identical_hash_skips_without_copy() {
        let (library, root) = temp_library();
        let outside = root.join("outside");
        fs::create_dir_all(&outside).unwrap();
        let first = outside.join("a.wav");
        let second = outside.join("b.wav");
        fs::write(&first, b"same-bytes").unwrap();
        fs::write(&second, b"same-bytes").unwrap();

        let added_at = 1;
        let first_out = ingest_path(
            None,
            &library,
            None,
            &first,
            &wav_meta(),
            added_at,
            IngestOptions::default(),
        )
        .unwrap();
        assert!(matches!(first_out, IngestOutcome::Added { .. }));
        let second_out = ingest_path(
            None,
            &library,
            None,
            &second,
            &wav_meta(),
            added_at,
            IngestOptions::default(),
        )
        .unwrap();
        assert!(matches!(second_out, IngestOutcome::Skipped));
        assert!(second.is_file());
        assert_eq!(library.list_tracks().unwrap().len(), 1);
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn failed_stage_leaves_no_incoming() {
        let (library, root) = temp_library();
        let missing = root.join("outside").join("nope.wav");
        let result = ingest_path(
            None,
            &library,
            None,
            &missing,
            &wav_meta(),
            1,
            IngestOptions::default(),
        );
        assert!(result.is_err());
        let incoming = library.incoming_dir();
        let entries = fs::read_dir(&incoming).unwrap().count();
        assert_eq!(entries, 0);
        let _ = fs::remove_dir_all(&root);
    }
}
