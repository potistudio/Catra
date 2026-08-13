use super::content::{
    map_content_row_with_song_id, resolve_artwork_path, CONTENT_QUERY_WITH_SONG, RekordboxContent,
};
use super::db::MasterDatabase;
use super::ids::{new_uuid, unused_numeric_id};
use super::write::{now_local, timestamp_sql, WriteSession};
use rusqlite::{params, Row};
use serde::Serialize;
use std::path::Path;

pub const PLAYLIST_ATTR: i32 = 0;
pub const FOLDER_ATTR: i32 = 1;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RekordboxPlaylist {
    pub id: String,
    pub name: String,
    pub attribute: i32,
    pub parent_id: Option<String>,
    pub seq: i32,
}

impl MasterDatabase {
    pub fn list_playlists(&self) -> Result<Vec<RekordboxPlaylist>, String> {
        let mut stmt = self
            .conn()
            .prepare(
                "SELECT ID, Name, Attribute, ParentID, Seq
                 FROM djmdPlaylist
                 ORDER BY ParentID, Seq",
            )
            .map_err(|error| error.to_string())?;

        let rows = stmt
            .query_map([], map_playlist_row)
            .map_err(|error| error.to_string())?;

        rows.collect::<Result<Vec<_>, _>>()
            .map_err(|error| error.to_string())
    }

    pub fn get_playlist_content(
        &self,
        playlist_id: &str,
        db_dir: &Path,
    ) -> Result<Vec<RekordboxContent>, String> {
        let sql = format!(
            "{CONTENT_QUERY_WITH_SONG}
INNER JOIN djmdSongPlaylist sp ON sp.ContentID = c.ID
WHERE sp.PlaylistID = ?1
ORDER BY sp.TrackNo"
        );

        let mut stmt = self
            .conn()
            .prepare(&sql)
            .map_err(|error| error.to_string())?;

        let rows = stmt
            .query_map(params![playlist_id], map_content_row_with_song_id)
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

pub fn create_playlist(
    name: String,
    parent_id: Option<String>,
) -> Result<RekordboxPlaylist, String> {
    create_playlist_entry(name, parent_id, PLAYLIST_ATTR)
}

pub fn create_playlist_folder(
    name: String,
    parent_id: Option<String>,
) -> Result<RekordboxPlaylist, String> {
    create_playlist_entry(name, parent_id, FOLDER_ATTR)
}

fn create_playlist_entry(
    name: String,
    parent_id: Option<String>,
    attribute: i32,
) -> Result<RekordboxPlaylist, String> {
    let name = name.trim().to_string();
    if name.is_empty() {
        return Err("playlist name must not be empty".to_string());
    }

    let mut session = WriteSession::open()?;
    let parent = normalize_parent_storage(parent_id.as_deref());
    if parent != "root" {
        let attr: i32 = session
            .conn()
            .query_row(
                "SELECT Attribute FROM djmdPlaylist WHERE ID = ?1",
                params![parent.as_str()],
                |row| row.get(0),
            )
            .map_err(|_| format!("parent playlist {parent} was not found"))?;
        if attr != FOLDER_ATTR {
            return Err("parent must be a playlist folder".to_string());
        }
    }

    let sibling_count: i64 = session
        .conn()
        .query_row(
            "SELECT COUNT(*) FROM djmdPlaylist WHERE ParentID = ?1",
            params![parent.as_str()],
            |row| row.get(0),
        )
        .map_err(|error| error.to_string())?;
    let seq = (sibling_count as i32) + 1;
    let id = unused_numeric_id(session.conn(), "djmdPlaylist")?;
    let uuid = new_uuid();
    let now = now_local();
    let ts = timestamp_sql(now);

    session
        .conn()
        .execute(
            "INSERT INTO djmdPlaylist (
                ID, Seq, Name, ImagePath, Attribute, ParentID, SmartList,
                UUID, rb_data_status, rb_local_data_status, rb_local_deleted,
                rb_local_synced, usn, rb_local_usn, created_at, updated_at
             ) VALUES (
                ?1, ?2, ?3, NULL, ?4, ?5, NULL,
                ?6, 0, 0, 0, 0, NULL, NULL, ?7, ?7
             )",
            params![
                id.as_str(),
                seq,
                name.as_str(),
                attribute,
                parent.as_str(),
                uuid.as_str(),
                ts.as_str()
            ],
        )
        .map_err(|error| format!("failed to create playlist: {error}"))?;

    session.usn.track("djmdPlaylist", id.clone());
    if let Some(xml) = session.playlist_xml.as_mut() {
        xml.add(&id, &parent, attribute, now)?;
    }
    session.commit()?;

    Ok(RekordboxPlaylist {
        id,
        name,
        attribute,
        parent_id: normalize_parent_id(Some(parent)),
        seq,
    })
}

pub fn rename_playlist(id: String, name: String) -> Result<RekordboxPlaylist, String> {
    let name = name.trim().to_string();
    if name.is_empty() {
        return Err("playlist name must not be empty".to_string());
    }

    let mut session = WriteSession::open()?;
    let now = now_local();
    let ts = timestamp_sql(now);
    let updated = session
        .conn()
        .execute(
            "UPDATE djmdPlaylist SET Name = ?1, updated_at = ?2 WHERE ID = ?3",
            params![name.as_str(), ts.as_str(), id.as_str()],
        )
        .map_err(|error| error.to_string())?;
    if updated == 0 {
        return Err(format!("playlist {id} was not found"));
    }

    session.usn.track("djmdPlaylist", id.clone());
    if let Some(xml) = session.playlist_xml.as_mut() {
        xml.update_timestamp(&id, now)?;
    }

    let playlist = fetch_playlist(session.conn(), &id)?;
    session.commit()?;
    Ok(playlist)
}

pub fn delete_playlist(id: String) -> Result<(), String> {
    let mut session = WriteSession::open()?;
    if SPECIAL_IDS.contains(&id.as_str()) {
        return Err(format!("cannot delete special playlist {id}"));
    }

    let parent = session
        .conn()
        .query_row(
            "SELECT ParentID FROM djmdPlaylist WHERE ID = ?1",
            params![id.as_str()],
            |row| row.get::<_, Option<String>>(0),
        )
        .map_err(|_| format!("playlist {id} was not found"))?;
    let parent_storage = normalize_parent_storage(parent.as_deref());

    let mut stack = vec![id.clone()];
    let mut to_delete = Vec::new();
    while let Some(current) = stack.pop() {
        let children: Vec<String> = {
            let mut stmt = session
                .conn()
                .prepare("SELECT ID FROM djmdPlaylist WHERE ParentID = ?1")
                .map_err(|error| error.to_string())?;
            let rows = stmt
                .query_map(params![current.as_str()], |row| row.get(0))
                .map_err(|error| error.to_string())?;
            rows.collect::<Result<Vec<_>, _>>()
                .map_err(|error| error.to_string())?
        };
        stack.extend(children);
        to_delete.push(current);
    }

    // Delete deepest nodes first.
    to_delete.reverse();
    for playlist_id in &to_delete {
        let song_ids: Vec<String> = {
            let mut stmt = session
                .conn()
                .prepare("SELECT ID FROM djmdSongPlaylist WHERE PlaylistID = ?1")
                .map_err(|error| error.to_string())?;
            let rows = stmt
                .query_map(params![playlist_id.as_str()], |row| row.get(0))
                .map_err(|error| error.to_string())?;
            rows.collect::<Result<Vec<_>, _>>()
                .map_err(|error| error.to_string())?
        };
        session
            .usn
            .track_many("djmdSongPlaylist", song_ids.clone());
        session
            .conn()
            .execute(
                "DELETE FROM djmdSongPlaylist WHERE PlaylistID = ?1",
                params![playlist_id.as_str()],
            )
            .map_err(|error| error.to_string())?;

        session.usn.track("djmdPlaylist", playlist_id.clone());
        session
            .conn()
            .execute(
                "DELETE FROM djmdPlaylist WHERE ID = ?1",
                params![playlist_id.as_str()],
            )
            .map_err(|error| error.to_string())?;

        if let Some(xml) = session.playlist_xml.as_mut() {
            xml.remove(playlist_id)?;
        }
    }

    {
        let (conn, usn) = session.conn_and_usn();
        resequence_siblings(conn, &parent_storage, usn)?;
    }
    session.commit()?;
    Ok(())
}

fn resequence_siblings(
    conn: &rusqlite::Connection,
    parent: &str,
    usn: &mut super::registry::UsnBuffer,
) -> Result<(), String> {
    let siblings: Vec<String> = {
        let mut stmt = conn
            .prepare("SELECT ID FROM djmdPlaylist WHERE ParentID = ?1 ORDER BY Seq, ID")
            .map_err(|error| error.to_string())?;
        let rows = stmt
            .query_map(params![parent], |row| row.get(0))
            .map_err(|error| error.to_string())?;
        rows.collect::<Result<Vec<_>, _>>()
            .map_err(|error| error.to_string())?
    };
    let now = now_local();
    let ts = timestamp_sql(now);
    for (index, sibling_id) in siblings.iter().enumerate() {
        let seq = (index as i32) + 1;
        conn.execute(
            "UPDATE djmdPlaylist SET Seq = ?1, updated_at = ?2 WHERE ID = ?3",
            params![seq, ts.as_str(), sibling_id.as_str()],
        )
        .map_err(|error| error.to_string())?;
        usn.track("djmdPlaylist", sibling_id.clone());
    }
    Ok(())
}

const SPECIAL_IDS: &[&str] = &["100000", "200000"];

pub fn add_to_playlist(
    playlist_id: String,
    content_id: String,
    track_no: Option<i32>,
) -> Result<String, String> {
    let mut session = WriteSession::open()?;
    let attr: i32 = session
        .conn()
        .query_row(
            "SELECT Attribute FROM djmdPlaylist WHERE ID = ?1",
            params![playlist_id.as_str()],
            |row| row.get(0),
        )
        .map_err(|_| format!("playlist {playlist_id} was not found"))?;
    if attr != PLAYLIST_ATTR {
        return Err("target must be a normal playlist".to_string());
    }

    let exists: bool = session
        .conn()
        .prepare("SELECT 1 FROM djmdContent WHERE ID = ?1 LIMIT 1")
        .map_err(|error| error.to_string())?
        .exists(params![content_id.as_str()])
        .map_err(|error| error.to_string())?;
    if !exists {
        return Err(format!("content {content_id} was not found"));
    }

    let count: i64 = session
        .conn()
        .query_row(
            "SELECT COUNT(*) FROM djmdSongPlaylist WHERE PlaylistID = ?1",
            params![playlist_id.as_str()],
            |row| row.get(0),
        )
        .map_err(|error| error.to_string())?;

    let insert_track_no = match track_no {
        Some(value) if value < 1 => return Err("track number must be greater than 0".to_string()),
        Some(value) if value as i64 > count + 1 => {
            return Err(format!(
                "track number too high, playlist contains {count} items"
            ))
        }
        Some(value) => value,
        None => (count as i32) + 1,
    };

    let now = now_local();
    let ts = timestamp_sql(now);

    if (insert_track_no as i64) <= count {
        let shifted: Vec<String> = {
            let mut stmt = session
                .conn()
                .prepare(
                    "SELECT ID FROM djmdSongPlaylist
                     WHERE PlaylistID = ?1 AND TrackNo >= ?2
                     ORDER BY TrackNo DESC",
                )
                .map_err(|error| error.to_string())?;
            let rows = stmt
                .query_map(params![playlist_id.as_str(), insert_track_no], |row| {
                    row.get(0)
                })
                .map_err(|error| error.to_string())?;
            rows.collect::<Result<Vec<_>, _>>()
                .map_err(|error| error.to_string())?
        };
        for song_id in &shifted {
            session
                .conn()
                .execute(
                    "UPDATE djmdSongPlaylist SET TrackNo = TrackNo + 1, updated_at = ?1 WHERE ID = ?2",
                    params![ts.as_str(), song_id.as_str()],
                )
                .map_err(|error| error.to_string())?;
        }
        session.usn.track_many("djmdSongPlaylist", shifted);
    }

    let song_id = new_uuid();
    let uuid = new_uuid();
    session
        .conn()
        .execute(
            "INSERT INTO djmdSongPlaylist (
                ID, PlaylistID, ContentID, TrackNo,
                UUID, rb_data_status, rb_local_data_status, rb_local_deleted,
                rb_local_synced, usn, rb_local_usn, created_at, updated_at
             ) VALUES (
                ?1, ?2, ?3, ?4,
                ?5, 0, 0, 0, 0, NULL, NULL, ?6, ?6
             )",
            params![
                song_id.as_str(),
                playlist_id.as_str(),
                content_id.as_str(),
                insert_track_no,
                uuid.as_str(),
                ts.as_str()
            ],
        )
        .map_err(|error| format!("failed to add track to playlist: {error}"))?;

    session.usn.track("djmdSongPlaylist", song_id.clone());
    session.usn.track("djmdPlaylist", playlist_id.clone());
    session.conn().execute(
        "UPDATE djmdPlaylist SET updated_at = ?1 WHERE ID = ?2",
        params![ts.as_str(), playlist_id.as_str()],
    )
    .map_err(|error| error.to_string())?;
    if let Some(xml) = session.playlist_xml.as_mut() {
        xml.update_timestamp(&playlist_id, now)?;
    }

    session.commit()?;
    Ok(song_id)
}

pub fn remove_from_playlist(playlist_id: String, song_playlist_id: String) -> Result<(), String> {
    let mut session = WriteSession::open()?;
    let track_no: i32 = session
        .conn()
        .query_row(
            "SELECT TrackNo FROM djmdSongPlaylist WHERE ID = ?1 AND PlaylistID = ?2",
            params![song_playlist_id.as_str(), playlist_id.as_str()],
            |row| row.get(0),
        )
        .map_err(|_| {
            format!("song playlist entry {song_playlist_id} was not found in playlist {playlist_id}")
        })?;

    let now = now_local();
    let ts = timestamp_sql(now);

    session
        .conn()
        .execute(
            "DELETE FROM djmdSongPlaylist WHERE ID = ?1",
            params![song_playlist_id.as_str()],
        )
        .map_err(|error| error.to_string())?;
    session
        .usn
        .track("djmdSongPlaylist", song_playlist_id.clone());

    let shifted: Vec<String> = {
        let mut stmt = session
            .conn()
            .prepare(
                "SELECT ID FROM djmdSongPlaylist
                 WHERE PlaylistID = ?1 AND TrackNo > ?2
                 ORDER BY TrackNo",
            )
            .map_err(|error| error.to_string())?;
        let rows = stmt
            .query_map(params![playlist_id.as_str(), track_no], |row| row.get(0))
            .map_err(|error| error.to_string())?;
        rows.collect::<Result<Vec<_>, _>>()
            .map_err(|error| error.to_string())?
    };
    for song_id in &shifted {
        session
            .conn()
            .execute(
                "UPDATE djmdSongPlaylist SET TrackNo = TrackNo - 1, updated_at = ?1 WHERE ID = ?2",
                params![ts.as_str(), song_id.as_str()],
            )
            .map_err(|error| error.to_string())?;
    }
    session.usn.track_many("djmdSongPlaylist", shifted);

    session.conn().execute(
        "UPDATE djmdPlaylist SET updated_at = ?1 WHERE ID = ?2",
        params![ts.as_str(), playlist_id.as_str()],
    )
    .map_err(|error| error.to_string())?;
    session.usn.track("djmdPlaylist", playlist_id.clone());
    if let Some(xml) = session.playlist_xml.as_mut() {
        xml.update_timestamp(&playlist_id, now)?;
    }

    session.commit()?;
    Ok(())
}

pub fn move_song_in_playlist(
    playlist_id: String,
    song_playlist_id: String,
    new_track_no: i32,
) -> Result<(), String> {
    if new_track_no < 1 {
        return Err("track number must be greater than 0".to_string());
    }

    let mut session = WriteSession::open()?;
    let old_track_no: i32 = session
        .conn()
        .query_row(
            "SELECT TrackNo FROM djmdSongPlaylist WHERE ID = ?1 AND PlaylistID = ?2",
            params![song_playlist_id.as_str(), playlist_id.as_str()],
            |row| row.get(0),
        )
        .map_err(|_| {
            format!("song playlist entry {song_playlist_id} was not found in playlist {playlist_id}")
        })?;

    let count: i64 = session
        .conn()
        .query_row(
            "SELECT COUNT(*) FROM djmdSongPlaylist WHERE PlaylistID = ?1",
            params![playlist_id.as_str()],
            |row| row.get(0),
        )
        .map_err(|error| error.to_string())?;
    if new_track_no as i64 > count {
        return Err(format!(
            "track number too high, playlist contains {count} items"
        ));
    }
    if new_track_no == old_track_no {
        return Ok(());
    }

    let now = now_local();
    let ts = timestamp_sql(now);

    // Park the moved song at TrackNo 0 temporarily to avoid unique conflicts.
    session
        .conn()
        .execute(
            "UPDATE djmdSongPlaylist SET TrackNo = 0, updated_at = ?1 WHERE ID = ?2",
            params![ts.as_str(), song_playlist_id.as_str()],
        )
        .map_err(|error| error.to_string())?;

    if new_track_no > old_track_no {
        let shifted: Vec<String> = {
            let mut stmt = session
                .conn()
                .prepare(
                    "SELECT ID FROM djmdSongPlaylist
                     WHERE PlaylistID = ?1 AND TrackNo > ?2 AND TrackNo <= ?3
                     ORDER BY TrackNo",
                )
                .map_err(|error| error.to_string())?;
            let rows = stmt
                .query_map(
                    params![playlist_id.as_str(), old_track_no, new_track_no],
                    |row| row.get(0),
                )
                .map_err(|error| error.to_string())?;
            rows.collect::<Result<Vec<_>, _>>()
                .map_err(|error| error.to_string())?
        };
        for song_id in &shifted {
            session
                .conn()
                .execute(
                    "UPDATE djmdSongPlaylist SET TrackNo = TrackNo - 1, updated_at = ?1 WHERE ID = ?2",
                    params![ts.as_str(), song_id.as_str()],
                )
                .map_err(|error| error.to_string())?;
        }
        session.usn.track_many("djmdSongPlaylist", shifted);
    } else {
        let shifted: Vec<String> = {
            let mut stmt = session
                .conn()
                .prepare(
                    "SELECT ID FROM djmdSongPlaylist
                     WHERE PlaylistID = ?1 AND TrackNo >= ?2 AND TrackNo < ?3
                     ORDER BY TrackNo DESC",
                )
                .map_err(|error| error.to_string())?;
            let rows = stmt
                .query_map(
                    params![playlist_id.as_str(), new_track_no, old_track_no],
                    |row| row.get(0),
                )
                .map_err(|error| error.to_string())?;
            rows.collect::<Result<Vec<_>, _>>()
                .map_err(|error| error.to_string())?
        };
        for song_id in &shifted {
            session
                .conn()
                .execute(
                    "UPDATE djmdSongPlaylist SET TrackNo = TrackNo + 1, updated_at = ?1 WHERE ID = ?2",
                    params![ts.as_str(), song_id.as_str()],
                )
                .map_err(|error| error.to_string())?;
        }
        session.usn.track_many("djmdSongPlaylist", shifted);
    }

    session
        .conn()
        .execute(
            "UPDATE djmdSongPlaylist SET TrackNo = ?1, updated_at = ?2 WHERE ID = ?3",
            params![new_track_no, ts.as_str(), song_playlist_id.as_str()],
        )
        .map_err(|error| error.to_string())?;
    session.usn.track("djmdSongPlaylist", song_playlist_id);

    session.conn().execute(
        "UPDATE djmdPlaylist SET updated_at = ?1 WHERE ID = ?2",
        params![ts.as_str(), playlist_id.as_str()],
    )
    .map_err(|error| error.to_string())?;
    session.usn.track("djmdPlaylist", playlist_id.clone());
    if let Some(xml) = session.playlist_xml.as_mut() {
        xml.update_timestamp(&playlist_id, now)?;
    }

    session.commit()?;
    Ok(())
}

fn fetch_playlist(conn: &rusqlite::Connection, id: &str) -> Result<RekordboxPlaylist, String> {
    conn.query_row(
        "SELECT ID, Name, Attribute, ParentID, Seq FROM djmdPlaylist WHERE ID = ?1",
        params![id],
        map_playlist_row,
    )
    .map_err(|error| error.to_string())
}

fn map_playlist_row(row: &Row<'_>) -> rusqlite::Result<RekordboxPlaylist> {
    let parent_id: Option<String> = row.get(3)?;
    Ok(RekordboxPlaylist {
        id: row.get(0)?,
        name: row.get(1)?,
        attribute: row.get(2)?,
        parent_id: normalize_parent_id(parent_id),
        seq: row.get::<_, Option<i32>>(4)?.unwrap_or(0),
    })
}

fn normalize_parent_id(parent_id: Option<String>) -> Option<String> {
    parent_id.and_then(|value| {
        let trimmed = value.trim();
        if trimmed.is_empty() || trimmed.eq_ignore_ascii_case("root") || trimmed == "0" {
            None
        } else {
            Some(trimmed.to_string())
        }
    })
}

fn normalize_parent_storage(parent_id: Option<&str>) -> String {
    match parent_id.map(str::trim) {
        None | Some("") | Some("0") | Some("root") | Some("ROOT") => "root".to_string(),
        Some(value) => value.to_string(),
    }
}
