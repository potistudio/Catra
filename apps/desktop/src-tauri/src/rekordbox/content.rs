use super::config::load_config;
use super::db::MasterDatabase;
use super::ids::{new_uuid, unused_numeric_id};
use super::write::{now_local, timestamp_sql, WriteSession};
use lofty::config::ParseOptions;
use lofty::file::{AudioFile, TaggedFileExt};
use lofty::probe::Probe;
use lofty::tag::{Accessor, ItemKey};
use rusqlite::{params, OptionalExtension, Row};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RekordboxContent {
    pub id: String,
    pub folder_path: String,
    pub file_name: Option<String>,
    pub title: Option<String>,
    pub artist: Option<String>,
    pub album: Option<String>,
    pub genre: Option<String>,
    pub bpm: Option<f32>,
    pub length_secs: Option<i32>,
    pub track_no: Option<i32>,
    pub bit_rate: Option<i32>,
    pub bit_depth: Option<i32>,
    pub comment: Option<String>,
    pub file_type: Option<i32>,
    pub rating: Option<i32>,
    pub release_year: Option<i32>,
    pub key: Option<String>,
    pub remixer: Option<String>,
    pub label: Option<String>,
    pub composer: Option<String>,
    pub file_size: Option<i32>,
    pub disc_no: Option<i32>,
    pub artwork_path: Option<String>,
    /// `djmdSongPlaylist.ID` when loaded via playlist membership.
    pub song_playlist_id: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RekordboxContentUpdate {
    pub title: Option<String>,
    pub artist: Option<String>,
    pub album: Option<String>,
    pub genre: Option<String>,
    pub comment: Option<String>,
    pub bpm: Option<f32>,
    pub rating: Option<i32>,
    pub track_no: Option<i32>,
    pub release_year: Option<i32>,
    pub key: Option<String>,
}

pub(crate) const CONTENT_QUERY: &str = "
SELECT
    c.ID,
    c.FolderPath,
    c.FileNameL,
    c.Title,
    artist.Name,
    album.Name,
    genre.Name,
    c.BPM,
    c.Length,
    c.TrackNo,
    c.BitRate,
    c.BitDepth,
    c.Commnt,
    c.FileType,
    c.Rating,
    c.ReleaseYear,
    djmd_key.ScaleName,
    remixer.Name,
    label.Name,
    composer.Name,
    c.FileSize,
    c.DiscNo,
    c.ImagePath
FROM djmdContent c
LEFT JOIN djmdArtist artist ON c.ArtistID = artist.ID
LEFT JOIN djmdAlbum album ON c.AlbumID = album.ID
LEFT JOIN djmdGenre genre ON c.GenreID = genre.ID
LEFT JOIN djmdKey djmd_key ON c.KeyID = djmd_key.ID
LEFT JOIN djmdArtist remixer ON c.RemixerID = remixer.ID
LEFT JOIN djmdLabel label ON c.LabelID = label.ID
LEFT JOIN djmdArtist composer ON c.ComposerID = composer.ID
";

pub(crate) const CONTENT_QUERY_WITH_SONG: &str = "
SELECT
    sp.ID,
    c.ID,
    c.FolderPath,
    c.FileNameL,
    c.Title,
    artist.Name,
    album.Name,
    genre.Name,
    c.BPM,
    c.Length,
    c.TrackNo,
    c.BitRate,
    c.BitDepth,
    c.Commnt,
    c.FileType,
    c.Rating,
    c.ReleaseYear,
    djmd_key.ScaleName,
    remixer.Name,
    label.Name,
    composer.Name,
    c.FileSize,
    c.DiscNo,
    c.ImagePath
FROM djmdContent c
LEFT JOIN djmdArtist artist ON c.ArtistID = artist.ID
LEFT JOIN djmdAlbum album ON c.AlbumID = album.ID
LEFT JOIN djmdGenre genre ON c.GenreID = genre.ID
LEFT JOIN djmdKey djmd_key ON c.KeyID = djmd_key.ID
LEFT JOIN djmdArtist remixer ON c.RemixerID = remixer.ID
LEFT JOIN djmdLabel label ON c.LabelID = label.ID
LEFT JOIN djmdArtist composer ON c.ComposerID = composer.ID
";

impl MasterDatabase {
    pub fn get_content(
        &self,
        id: Option<&str>,
        db_dir: &Path,
    ) -> Result<Vec<RekordboxContent>, String> {
        let sql = match id {
            Some(_) => format!("{CONTENT_QUERY} WHERE c.ID = ?1"),
            None => CONTENT_QUERY.to_string(),
        };

        let mut stmt = self
            .conn()
            .prepare(&sql)
            .map_err(|error| error.to_string())?;

        let rows = match id {
            Some(content_id) => stmt.query_map(params![content_id], map_content_row),
            None => stmt.query_map([], map_content_row),
        }
        .map_err(|error| error.to_string())?;

        let mut contents = rows
            .collect::<Result<Vec<_>, _>>()
            .map_err(|error| error.to_string())?;

        for content in &mut contents {
            content.artwork_path =
                resolve_artwork_path(db_dir, content.artwork_path.as_deref());
        }

        Ok(contents)
    }
}

pub fn add_content(path: String, title: Option<String>) -> Result<RekordboxContent, String> {
    let path = PathBuf::from(path);
    if !path.is_file() {
        return Err(format!("file not found: {}", path.display()));
    }

    let folder_path = path.to_string_lossy().replace('/', "\\");

    {
        let db = MasterDatabase::open()?;
        let existing: Option<(String, i32)> = db
            .conn()
            .query_row(
                "SELECT ID, IFNULL(rb_local_deleted, 0) FROM djmdContent
                 WHERE FolderPath = ?1
                 ORDER BY IFNULL(rb_local_deleted, 0) ASC
                 LIMIT 1",
                params![folder_path.as_str()],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .optional()
            .map_err(|error| format!("failed to lookup existing content: {error}"))?;
        if let Some((id, deleted)) = existing {
            if deleted != 0 {
                let source = path.to_string_lossy().into_owned();
                return restore_content(&id, Some(&source));
            }
            let db_dir = load_config()?
                .db_dir
                .ok_or_else(|| "Rekordbox database directory was not found".to_string())?;
            let mut contents = db.get_content(Some(&id), &db_dir)?;
            if let Some(content) = contents.pop() {
                return Ok(content);
            }
        }
    }

    let file_name = path
        .file_name()
        .and_then(|value| value.to_str())
        .map(str::to_string)
        .ok_or_else(|| "invalid file name".to_string())?;
    let file_size = std::fs::metadata(&path)
        .map(|meta| meta.len().min(i32::MAX as u64) as i32)
        .ok();
    let file_type = file_type_from_path(&path);
    let meta = read_file_metadata(&path);
    let title = title
        .filter(|value| !value.trim().is_empty())
        .or(meta.title.clone())
        .or_else(|| {
            path.file_stem()
                .and_then(|value| value.to_str())
                .map(str::to_string)
        });

    let mut session = WriteSession::open()?;
    let id = unused_numeric_id(session.conn(), "djmdContent")?;
    let uuid = new_uuid();
    let now = now_local();
    let ts = timestamp_sql(now);

    let artist_id = {
        let (conn, usn) = session.conn_and_usn();
        ensure_named_row(conn, usn, "djmdArtist", "Name", meta.artist.as_deref(), &ts)?
    };
    let album_id = {
        let (conn, usn) = session.conn_and_usn();
        ensure_named_row(conn, usn, "djmdAlbum", "Name", meta.album.as_deref(), &ts)?
    };
    let genre_id = {
        let (conn, usn) = session.conn_and_usn();
        ensure_named_row(conn, usn, "djmdGenre", "Name", meta.genre.as_deref(), &ts)?
    };
    let key_id = {
        let (conn, usn) = session.conn_and_usn();
        ensure_named_row(conn, usn, "djmdKey", "ScaleName", meta.key.as_deref(), &ts)?
    };

    let bpm_db = meta.bpm.map(|value| (value * 100.0).round() as i32);
    let rating_db = denormalize_rating(meta.rating);

    session
        .conn()
        .execute(
            "INSERT INTO djmdContent (
                ID, FolderPath, FileNameL, Title,
                ArtistID, AlbumID, GenreID, BPM, Length,
                BitRate, FileType, Rating, KeyID, FileSize, SampleRate,
                OrgFolderPath, DateCreated,
                UUID, rb_data_status, rb_local_data_status,
                rb_local_deleted, rb_local_synced, created_at, updated_at
             ) VALUES (
                ?1, ?2, ?3, ?4,
                ?5, ?6, ?7, ?8, ?9,
                ?10, ?11, ?12, ?13, ?14, ?15,
                ?2, ?16,
                ?17, 0, 0, 0, 0, ?16, ?16
             )",
            params![
                id.as_str(),
                folder_path.as_str(),
                file_name.as_str(),
                title.as_deref(),
                artist_id.as_deref(),
                album_id.as_deref(),
                genre_id.as_deref(),
                bpm_db,
                meta.length_secs,
                meta.bit_rate,
                file_type,
                rating_db,
                key_id.as_deref(),
                file_size,
                meta.sample_rate,
                ts.as_str(),
                uuid.as_str(),
            ],
        )
        .map_err(|error| format!("failed to add content: {error}"))?;

    session.usn.track("djmdContent", id.clone());
    let db_dir = session.db_dir.clone();
    session.commit()?;

    let db = MasterDatabase::open()?;
    let mut contents = db.get_content(Some(&id), &db_dir)?;
    contents
        .pop()
        .ok_or_else(|| "added content could not be reloaded".to_string())
}

pub fn update_content(
    id: String,
    fields: RekordboxContentUpdate,
) -> Result<RekordboxContent, String> {
    let mut session = WriteSession::open()?;
    let exists: bool = session
        .conn()
        .prepare("SELECT 1 FROM djmdContent WHERE ID = ?1 LIMIT 1")
        .map_err(|error| error.to_string())?
        .exists(params![id.as_str()])
        .map_err(|error| error.to_string())?;
    if !exists {
        return Err(format!("content {id} was not found"));
    }

    let now = now_local();
    let ts = timestamp_sql(now);

    if let Some(title) = fields.title.as_ref() {
        session
            .conn()
            .execute(
                "UPDATE djmdContent SET Title = ?1, updated_at = ?2 WHERE ID = ?3",
                params![title.as_str(), ts.as_str(), id.as_str()],
            )
            .map_err(|error| error.to_string())?;
    }
    if let Some(comment) = fields.comment.as_ref() {
        session
            .conn()
            .execute(
                "UPDATE djmdContent SET Commnt = ?1, updated_at = ?2 WHERE ID = ?3",
                params![comment.as_str(), ts.as_str(), id.as_str()],
            )
            .map_err(|error| error.to_string())?;
    }
    if let Some(bpm) = fields.bpm {
        let bpm_db = (bpm * 100.0).round() as i32;
        session
            .conn()
            .execute(
                "UPDATE djmdContent SET BPM = ?1, updated_at = ?2 WHERE ID = ?3",
                params![bpm_db, ts.as_str(), id.as_str()],
            )
            .map_err(|error| error.to_string())?;
    }
    if let Some(rating) = fields.rating {
        let rating_db = denormalize_rating(Some(rating)).unwrap_or(0);
        session
            .conn()
            .execute(
                "UPDATE djmdContent SET Rating = ?1, updated_at = ?2 WHERE ID = ?3",
                params![rating_db, ts.as_str(), id.as_str()],
            )
            .map_err(|error| error.to_string())?;
    }
    if let Some(track_no) = fields.track_no {
        session
            .conn()
            .execute(
                "UPDATE djmdContent SET TrackNo = ?1, updated_at = ?2 WHERE ID = ?3",
                params![track_no, ts.as_str(), id.as_str()],
            )
            .map_err(|error| error.to_string())?;
    }
    if let Some(release_year) = fields.release_year {
        session
            .conn()
            .execute(
                "UPDATE djmdContent SET ReleaseYear = ?1, updated_at = ?2 WHERE ID = ?3",
                params![release_year, ts.as_str(), id.as_str()],
            )
            .map_err(|error| error.to_string())?;
    }

    if let Some(artist) = fields.artist.as_ref() {
        let artist_id = {
            let (conn, usn) = session.conn_and_usn();
            ensure_named_row(conn, usn, "djmdArtist", "Name", Some(artist.as_str()), &ts)?
        };
        session
            .conn()
            .execute(
                "UPDATE djmdContent SET ArtistID = ?1, updated_at = ?2 WHERE ID = ?3",
                params![artist_id.as_deref(), ts.as_str(), id.as_str()],
            )
            .map_err(|error| error.to_string())?;
    }
    if let Some(album) = fields.album.as_ref() {
        let album_id = {
            let (conn, usn) = session.conn_and_usn();
            ensure_named_row(conn, usn, "djmdAlbum", "Name", Some(album.as_str()), &ts)?
        };
        session
            .conn()
            .execute(
                "UPDATE djmdContent SET AlbumID = ?1, updated_at = ?2 WHERE ID = ?3",
                params![album_id.as_deref(), ts.as_str(), id.as_str()],
            )
            .map_err(|error| error.to_string())?;
    }
    if let Some(genre) = fields.genre.as_ref() {
        let genre_id = {
            let (conn, usn) = session.conn_and_usn();
            ensure_named_row(conn, usn, "djmdGenre", "Name", Some(genre.as_str()), &ts)?
        };
        session
            .conn()
            .execute(
                "UPDATE djmdContent SET GenreID = ?1, updated_at = ?2 WHERE ID = ?3",
                params![genre_id.as_deref(), ts.as_str(), id.as_str()],
            )
            .map_err(|error| error.to_string())?;
    }
    if let Some(key) = fields.key.as_ref() {
        let key_id = {
            let (conn, usn) = session.conn_and_usn();
            ensure_named_row(conn, usn, "djmdKey", "ScaleName", Some(key.as_str()), &ts)?
        };
        session
            .conn()
            .execute(
                "UPDATE djmdContent SET KeyID = ?1, updated_at = ?2 WHERE ID = ?3",
                params![key_id.as_deref(), ts.as_str(), id.as_str()],
            )
            .map_err(|error| error.to_string())?;
    }

    session.usn.track("djmdContent", id.clone());
    let db_dir = session.db_dir.clone();
    session.commit()?;

    let db = MasterDatabase::open()?;
    let mut contents = db.get_content(Some(&id), &db_dir)?;
    contents
        .pop()
        .ok_or_else(|| "updated content could not be reloaded".to_string())
}

pub fn update_content_folder_path(id: &str, path: &str) -> Result<RekordboxContent, String> {
    let path = PathBuf::from(path);
    if !path.is_file() {
        return Err(format!("file not found: {}", path.display()));
    }
    let folder_path = path.to_string_lossy().replace('/', "\\");
    let file_name = path
        .file_name()
        .and_then(|value| value.to_str())
        .ok_or_else(|| "invalid file name".to_string())?;

    let mut session = WriteSession::open()?;
    let exists: bool = session
        .conn()
        .prepare("SELECT 1 FROM djmdContent WHERE ID = ?1 LIMIT 1")
        .map_err(|error| error.to_string())?
        .exists(params![id])
        .map_err(|error| error.to_string())?;
    if !exists {
        return Err(format!("content {id} was not found"));
    }

    let ts = timestamp_sql(now_local());
    session
        .conn()
        .execute(
            "UPDATE djmdContent SET FolderPath = ?1, FileNameL = ?2, updated_at = ?3 WHERE ID = ?4",
            params![folder_path.as_str(), file_name, ts.as_str(), id],
        )
        .map_err(|error| error.to_string())?;
    session.usn.track("djmdContent", id.to_string());
    let db_dir = session.db_dir.clone();
    session.commit()?;

    let db = MasterDatabase::open()?;
    let mut contents = db.get_content(Some(id), &db_dir)?;
    contents
        .pop()
        .ok_or_else(|| "updated content could not be reloaded".to_string())
}

/// Set `rb_local_deleted = 0` on Content and its `djmdSongPlaylist` rows.
/// When `path` is set, `FolderPath` and `FileNameL` are updated in the same write.
pub fn restore_content(id: &str, path: Option<&str>) -> Result<RekordboxContent, String> {
    let mut folder_path = None;
    let mut file_name = None;
    if let Some(path) = path {
        let path = PathBuf::from(path);
        if !path.is_file() {
            return Err(format!("file not found: {}", path.display()));
        }
        folder_path = Some(path.to_string_lossy().replace('/', "\\"));
        file_name = Some(
            path.file_name()
                .and_then(|value| value.to_str())
                .ok_or_else(|| "invalid file name".to_string())?
                .to_string(),
        );
    }

    let mut session = WriteSession::open()?;
    if !content_row_exists(session.conn(), id)? {
        return Err(format!("content {id} was not found"));
    }
    let ts = timestamp_sql(now_local());
    set_playlist_songs_deleted(&mut session, id, 0, &ts)?;
    session
        .conn()
        .execute(
            "UPDATE djmdContent
             SET rb_local_deleted = 0,
                 FolderPath = COALESCE(?1, FolderPath),
                 FileNameL = COALESCE(?2, FileNameL),
                 updated_at = ?3
             WHERE ID = ?4",
            params![
                folder_path.as_deref(),
                file_name.as_deref(),
                ts.as_str(),
                id
            ],
        )
        .map_err(|error| error.to_string())?;
    session.usn.track("djmdContent", id.to_string());
    let db_dir = session.db_dir.clone();
    session.commit()?;

    let db = MasterDatabase::open()?;
    let mut contents = db.get_content(Some(id), &db_dir)?;
    contents
        .pop()
        .ok_or_else(|| "restored content could not be reloaded".to_string())
}

pub fn delete_content(id: String) -> Result<(), String> {
    let mut session = WriteSession::open()?;
    if !content_row_exists(session.conn(), &id)? {
        return Err(format!("content {id} was not found"));
    }
    purge_content_rows(&mut session, &id)?;
    session.commit()?;
    Ok(())
}

pub(crate) fn purge_content_rows(session: &mut WriteSession, id: &str) -> Result<(), String> {
    let song_ids: Vec<String> = {
        let mut stmt = session
            .conn()
            .prepare("SELECT ID FROM djmdSongPlaylist WHERE ContentID = ?1")
            .map_err(|error| error.to_string())?;
        let rows = stmt
            .query_map(params![id], |row| row.get(0))
            .map_err(|error| error.to_string())?;
        rows.collect::<Result<Vec<_>, _>>()
            .map_err(|error| error.to_string())?
    };
    session.usn.track_many("djmdSongPlaylist", song_ids);
    session
        .conn()
        .execute(
            "DELETE FROM djmdSongPlaylist WHERE ContentID = ?1",
            params![id],
        )
        .map_err(|error| error.to_string())?;

    for table in [
        "djmdSongMyTag",
        "djmdSongHistory",
        "djmdSongRelatedTracks",
        "djmdCue",
    ] {
        let _ = session.conn().execute(
            &format!("DELETE FROM {table} WHERE ContentID = ?1"),
            params![id],
        );
    }

    session
        .conn()
        .execute("DELETE FROM djmdContent WHERE ID = ?1", params![id])
        .map_err(|error| error.to_string())?;
    session.usn.track("djmdContent", id.to_string());
    Ok(())
}

fn content_row_exists(conn: &rusqlite::Connection, id: &str) -> Result<bool, String> {
    conn.prepare("SELECT 1 FROM djmdContent WHERE ID = ?1 LIMIT 1")
        .map_err(|error| error.to_string())?
        .exists(params![id])
        .map_err(|error| error.to_string())
}

fn set_playlist_songs_deleted(
    session: &mut WriteSession,
    content_id: &str,
    deleted: i32,
    ts: &str,
) -> Result<(), String> {
    let song_ids: Vec<String> = {
        let mut stmt = session
            .conn()
            .prepare("SELECT ID FROM djmdSongPlaylist WHERE ContentID = ?1")
            .map_err(|error| error.to_string())?;
        let rows = stmt
            .query_map(params![content_id], |row| row.get(0))
            .map_err(|error| error.to_string())?;
        rows.collect::<Result<Vec<_>, _>>()
            .map_err(|error| error.to_string())?
    };
    if song_ids.is_empty() {
        return Ok(());
    }
    session
        .conn()
        .execute(
            "UPDATE djmdSongPlaylist SET rb_local_deleted = ?1, updated_at = ?2 WHERE ContentID = ?3",
            params![deleted, ts, content_id],
        )
        .map_err(|error| error.to_string())?;
    session.usn.track_many("djmdSongPlaylist", song_ids);
    Ok(())
}

fn ensure_named_row(
    conn: &rusqlite::Connection,
    usn: &mut super::registry::UsnBuffer,
    table: &'static str,
    name_column: &str,
    name: Option<&str>,
    ts: &str,
) -> Result<Option<String>, String> {
    let Some(name) = name.map(str::trim).filter(|value| !value.is_empty()) else {
        return Ok(None);
    };

    let existing: Option<String> = conn
        .query_row(
            &format!("SELECT ID FROM {table} WHERE {name_column} = ?1 LIMIT 1"),
            params![name],
            |row| row.get(0),
        )
        .optional()
        .map_err(|error| error.to_string())?;
    if let Some(id) = existing {
        return Ok(Some(id));
    }

    let id = unused_numeric_id(conn, table)?;
    let uuid = new_uuid();
    conn.execute(
        &format!(
            "INSERT INTO {table} (
                ID, {name_column}, UUID,
                rb_data_status, rb_local_data_status, rb_local_deleted, rb_local_synced,
                created_at, updated_at
             ) VALUES (
                ?1, ?2, ?3, 0, 0, 0, 0, ?4, ?4
             )"
        ),
        params![id.as_str(), name, uuid.as_str(), ts],
    )
    .map_err(|error| format!("failed to insert into {table}: {error}"))?;
    usn.track(table, id.clone());
    Ok(Some(id))
}

struct FileMeta {
    title: Option<String>,
    artist: Option<String>,
    album: Option<String>,
    genre: Option<String>,
    key: Option<String>,
    bpm: Option<f32>,
    rating: Option<i32>,
    length_secs: Option<i32>,
    bit_rate: Option<i32>,
    sample_rate: Option<i32>,
}

fn read_file_metadata(path: &Path) -> FileMeta {
    let fallback_title = path
        .file_stem()
        .and_then(|value| value.to_str())
        .map(str::to_string);
    let options = ParseOptions::new().max_junk_bytes(4096);
    let Ok(tagged_file) = Probe::open(path).and_then(|probe| probe.options(options).read()) else {
        return FileMeta {
            title: fallback_title,
            artist: None,
            album: None,
            genre: None,
            key: None,
            bpm: None,
            rating: None,
            length_secs: None,
            bit_rate: None,
            sample_rate: None,
        };
    };

    let properties = tagged_file.properties();
    let length_secs = Some(properties.duration().as_secs().min(i32::MAX as u64) as i32);
    let bit_rate = properties.audio_bitrate().map(|value| value as i32);
    let sample_rate = properties.sample_rate().map(|value| value as i32);

    if let Some(tag) = tagged_file.primary_tag() {
        return FileMeta {
            title: tag.title().map(|value| value.to_string()).or(fallback_title),
            artist: tag.artist().map(|value| value.to_string()),
            album: tag.album().map(|value| value.to_string()),
            genre: tag.genre().map(|value| value.to_string()),
            key: tag
                .get_string(&ItemKey::InitialKey)
                .map(|value| value.to_string()),
            bpm: tag
                .get_string(&ItemKey::Bpm)
                .or_else(|| tag.get_string(&ItemKey::IntegerBpm))
                .and_then(|value| value.parse::<f32>().ok()),
            rating: None,
            length_secs,
            bit_rate,
            sample_rate,
        };
    }

    FileMeta {
        title: fallback_title,
        artist: None,
        album: None,
        genre: None,
        key: None,
        bpm: None,
        rating: None,
        length_secs,
        bit_rate,
        sample_rate,
    }
}

fn file_type_from_path(path: &Path) -> Option<i32> {
    match path
        .extension()
        .and_then(|value| value.to_str())
        .map(|value| value.to_ascii_lowercase())
        .as_deref()
    {
        Some("mp3") => Some(1),
        Some("m4a") | Some("mp4") | Some("aac") => Some(4),
        Some("flac") => Some(5),
        Some("wav") => Some(11),
        Some("aif") | Some("aiff") => Some(12),
        _ => None,
    }
}

pub(crate) fn map_content_row(row: &Row<'_>) -> rusqlite::Result<RekordboxContent> {
    map_content_columns(row, 0, None)
}

pub(crate) fn map_content_row_with_song_id(row: &Row<'_>) -> rusqlite::Result<RekordboxContent> {
    let song_playlist_id: String = row.get(0)?;
    map_content_columns(row, 1, Some(song_playlist_id))
}

fn map_content_columns(
    row: &Row<'_>,
    offset: usize,
    song_playlist_id: Option<String>,
) -> rusqlite::Result<RekordboxContent> {
    let bpm_raw: Option<i32> = row.get(offset + 7)?;
    Ok(RekordboxContent {
        id: row.get(offset)?,
        folder_path: row.get(offset + 1)?,
        file_name: row.get(offset + 2)?,
        title: row.get(offset + 3)?,
        artist: row.get(offset + 4)?,
        album: row.get(offset + 5)?,
        genre: row.get(offset + 6)?,
        bpm: bpm_raw.map(|value| value as f32 / 100.0),
        length_secs: row.get(offset + 8)?,
        track_no: row.get(offset + 9)?,
        bit_rate: row.get(offset + 10)?,
        bit_depth: row.get(offset + 11)?,
        comment: row.get(offset + 12)?,
        file_type: row.get(offset + 13)?,
        rating: normalize_rating(row.get(offset + 14)?),
        release_year: row.get(offset + 15)?,
        key: row.get(offset + 16)?,
        remixer: row.get(offset + 17)?,
        label: row.get(offset + 18)?,
        composer: row.get(offset + 19)?,
        file_size: row.get(offset + 20)?,
        disc_no: row.get(offset + 21)?,
        artwork_path: row.get(offset + 22)?,
        song_playlist_id,
    })
}

/// Map `djmdContent.Rating` onto the 0-255 scale used by the UI and ID3 POPM.
///
/// `master.db` stores star counts 0-5. Rekordbox XML uses 0/51/102/153/204/255.
fn normalize_rating(rating: Option<i32>) -> Option<i32> {
    match rating {
        None => None,
        Some(stars) if (0..=5).contains(&stars) => Some(stars * 51),
        Some(value) => Some(value.clamp(0, 255)),
    }
}

fn denormalize_rating(rating: Option<i32>) -> Option<i32> {
    match rating {
        None => None,
        Some(value) if (0..=5).contains(&value) => Some(value),
        Some(value) => Some(((value.clamp(0, 255) as f32) / 51.0).round() as i32),
    }
}

/// Resolve Rekordbox `ImagePath` to an absolute file path under the DB directory.
pub(crate) fn resolve_artwork_path(db_dir: &Path, image_path: Option<&str>) -> Option<String> {
    let relative = normalize_image_path(image_path?)?;
    let path = Path::new(relative);

    let candidates: Vec<PathBuf> = if path.is_absolute() {
        vec![path.to_path_buf()]
    } else {
        // ImagePath is stored like `/PIONEER/Artwork/.../artwork.jpg` (share-relative).
        vec![db_dir.join("share").join(path), db_dir.join(path)]
    };

    for candidate in candidates {
        if let Some(resolved) = prefer_medium_artwork(&candidate) {
            return Some(resolved.to_string_lossy().into_owned());
        }
    }

    None
}

fn normalize_image_path(image_path: &str) -> Option<&str> {
    let trimmed = image_path.trim().trim_start_matches(['/', '\\']);
    if trimmed.is_empty() {
        None
    } else {
        Some(trimmed)
    }
}

fn prefer_medium_artwork(path: &Path) -> Option<PathBuf> {
    if let (Some(stem), Some(ext)) = (
        path.file_stem().and_then(|value| value.to_str()),
        path.extension().and_then(|value| value.to_str()),
    ) {
        if !stem.ends_with("_m") && !stem.ends_with("_s") {
            let medium = path.with_file_name(format!("{stem}_m.{ext}"));
            if medium.is_file() {
                return Some(medium);
            }
        }
    }

    path.is_file().then(|| path.to_path_buf())
}

#[cfg(test)]
mod tests {
    use super::{normalize_rating, prefer_medium_artwork, resolve_artwork_path};
    use std::fs;
    use std::path::PathBuf;

    #[test]
    fn normalizes_star_count_rating_to_255_scale() {
        assert_eq!(normalize_rating(None), None);
        assert_eq!(normalize_rating(Some(0)), Some(0));
        assert_eq!(normalize_rating(Some(1)), Some(51));
        assert_eq!(normalize_rating(Some(2)), Some(102));
        assert_eq!(normalize_rating(Some(3)), Some(153));
        assert_eq!(normalize_rating(Some(4)), Some(204));
        assert_eq!(normalize_rating(Some(5)), Some(255));
    }

    #[test]
    fn preserves_xml_style_255_scale_rating() {
        assert_eq!(normalize_rating(Some(51)), Some(51));
        assert_eq!(normalize_rating(Some(255)), Some(255));
        assert_eq!(normalize_rating(Some(300)), Some(255));
        assert_eq!(normalize_rating(Some(-1)), Some(0));
    }

    #[test]
    fn resolves_leading_slash_share_relative_path() {
        let root = std::env::temp_dir().join(format!(
            "catra-rb-art-slash-{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let art_dir = root.join("share/PIONEER/Artwork/000/abc");
        fs::create_dir_all(&art_dir).unwrap();
        let jpg = art_dir.join("artwork.jpg");
        let medium = art_dir.join("artwork_m.jpg");
        fs::write(&jpg, b"full").unwrap();
        fs::write(&medium, b"med").unwrap();

        let resolved =
            resolve_artwork_path(&root, Some("/PIONEER/Artwork/000/abc/artwork.jpg"))
                .expect("resolved");
        assert_eq!(PathBuf::from(resolved), medium);

        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn resolves_relative_image_path_under_share() {
        let root = std::env::temp_dir().join(format!(
            "catra-rb-art-{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let art_dir = root.join("share/PIONEER/Artwork/000/abc");
        fs::create_dir_all(&art_dir).unwrap();
        let jpg = art_dir.join("artwork.jpg");
        let medium = art_dir.join("artwork_m.jpg");
        fs::write(&jpg, b"full").unwrap();
        fs::write(&medium, b"med").unwrap();

        let resolved =
            resolve_artwork_path(&root, Some("share/PIONEER/Artwork/000/abc/artwork.jpg"))
                .expect("resolved");
        assert_eq!(PathBuf::from(resolved), medium);

        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn prefers_medium_when_present() {
        let root = std::env::temp_dir().join(format!(
            "catra-rb-art-med-{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        fs::create_dir_all(&root).unwrap();
        let jpg = root.join("artwork.jpg");
        let medium = root.join("artwork_m.jpg");
        fs::write(&jpg, b"full").unwrap();
        fs::write(&medium, b"med").unwrap();
        assert_eq!(prefer_medium_artwork(&jpg), Some(medium));
        let _ = fs::remove_dir_all(root);
    }
}
