use super::duplicate::is_same_track;
use super::paths::{
    clear_incoming, ensure_library_layout, resolve_library_root, to_absolute, to_relative,
};
use rusqlite::{params, Connection, Row};
use serde::Serialize;
use std::path::{Path, PathBuf};
use std::sync::{Mutex, MutexGuard};
use tauri::{AppHandle, Manager};

fn lock_conn(conn: &Mutex<Connection>) -> Result<MutexGuard<'_, Connection>, rusqlite::Error> {
    conn.lock().map_err(|_| {
        rusqlite::Error::SqliteFailure(
            rusqlite::ffi::Error::new(rusqlite::ffi::SQLITE_INTERNAL),
            Some("database lock poisoned".to_string()),
        )
    })
}

const TRACK_COLUMNS: &str = "id, path, title, artist, album, duration_ms, bpm, bitrate_kbps,
        genre, key_name, rating, artwork_path, source, converted, added_at,
        content_hash, trashed_at, parent_track_id, format_group_id, rekordbox_content_id";

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Track {
    pub id: i64,
    pub path: String,
    pub title: Option<String>,
    pub artist: Option<String>,
    pub album: Option<String>,
    pub duration_ms: Option<u64>,
    pub bpm: Option<f32>,
    pub bitrate_kbps: Option<u32>,
    pub genre: Option<String>,
    pub key: Option<String>,
    pub rating: Option<u8>,
    pub artwork_path: Option<String>,
    pub source: Option<String>,
    pub converted: bool,
    pub added_at: i64,
    pub content_hash: Option<String>,
    pub trashed_at: Option<i64>,
    pub parent_track_id: Option<i64>,
    pub format_group_id: Option<String>,
    pub rekordbox_content_id: Option<String>,
    #[serde(skip)]
    pub stored_path: String,
    #[serde(skip)]
    pub stored_artwork_path: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ScanResult {
    pub added: u32,
    pub skipped: u32,
}

pub struct NewTrack<'a> {
    pub path: &'a str,
    pub title: Option<&'a str>,
    pub artist: Option<&'a str>,
    pub album: Option<&'a str>,
    pub duration_ms: Option<u64>,
    pub bpm: Option<f32>,
    pub bitrate_kbps: Option<u32>,
    pub genre: Option<&'a str>,
    pub key: Option<&'a str>,
    pub rating: Option<u8>,
    pub artwork_path: Option<&'a str>,
    pub source: Option<&'a str>,
    pub converted: bool,
    pub added_at: i64,
    pub content_hash: Option<&'a str>,
    pub parent_track_id: Option<i64>,
    pub format_group_id: Option<&'a str>,
    pub rekordbox_content_id: Option<&'a str>,
}

pub struct LibraryState {
    conn: Mutex<Connection>,
    root: PathBuf,
    artwork_dir: PathBuf,
}

impl LibraryState {
    pub fn new(root: PathBuf) -> Result<Self, rusqlite::Error> {
        ensure_library_layout(&root).map_err(|error| {
            rusqlite::Error::SqliteFailure(
                rusqlite::ffi::Error::new(rusqlite::ffi::SQLITE_CANTOPEN),
                Some(error),
            )
        })?;
        clear_incoming(&root);

        let artwork_dir = root.join("artwork");
        let db_path = root.join("library.db");
        if let Some(parent) = db_path.parent() {
            std::fs::create_dir_all(parent).ok();
        }
        std::fs::create_dir_all(&artwork_dir).ok();

        let conn = Connection::open(db_path)?;
        conn.busy_timeout(std::time::Duration::from_secs(5))?;
        conn.pragma_update(None, "cipher_plaintext_header_size", 0)
            .ok();
        conn.execute_batch(
            "
            PRAGMA journal_mode=WAL;
            CREATE TABLE IF NOT EXISTS tracks (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                path TEXT NOT NULL UNIQUE,
                title TEXT,
                artist TEXT,
                album TEXT,
                duration_ms INTEGER,
                bpm REAL,
                bitrate_kbps INTEGER,
                genre TEXT,
                key_name TEXT,
                rating INTEGER,
                artwork_path TEXT,
                source TEXT,
                converted INTEGER NOT NULL DEFAULT 0,
                added_at INTEGER NOT NULL,
                content_hash TEXT,
                trashed_at INTEGER,
                parent_track_id INTEGER,
                format_group_id TEXT,
                rekordbox_content_id TEXT
            );
            CREATE INDEX IF NOT EXISTS idx_tracks_artist ON tracks(artist);
            CREATE INDEX IF NOT EXISTS idx_tracks_title ON tracks(title);
            CREATE INDEX IF NOT EXISTS idx_tracks_content_hash ON tracks(content_hash);
            CREATE INDEX IF NOT EXISTS idx_tracks_trashed_at ON tracks(trashed_at);
            CREATE INDEX IF NOT EXISTS idx_tracks_parent ON tracks(parent_track_id);
            ",
        )?;
        migrate(&conn)?;

        Ok(Self {
            conn: Mutex::new(conn),
            root,
            artwork_dir,
        })
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    pub fn artwork_dir(&self) -> &Path {
        &self.artwork_dir
    }

    pub fn incoming_dir(&self) -> PathBuf {
        self.root.join("incoming")
    }

    pub fn absolute_path(&self, relative: &str) -> PathBuf {
        to_absolute(&self.root, relative)
    }

    pub fn absolute_string(&self, relative: &str) -> String {
        self.absolute_path(relative).to_string_lossy().into_owned()
    }

    pub fn relative_of(&self, path: &Path) -> Option<String> {
        to_relative(&self.root, path)
    }

    fn expand_track(&self, mut track: Track) -> Track {
        track.path = self.absolute_string(&track.stored_path);
        track.artwork_path = track
            .stored_artwork_path
            .as_ref()
            .map(|relative| self.absolute_string(relative));
        track
    }

    pub fn list_tracks(&self) -> Result<Vec<Track>, rusqlite::Error> {
        self.list_tracks_filtered(false)
    }

    pub fn list_trashed(&self) -> Result<Vec<Track>, rusqlite::Error> {
        self.list_tracks_filtered(true)
    }

    fn list_tracks_filtered(&self, trashed: bool) -> Result<Vec<Track>, rusqlite::Error> {
        let conn = lock_conn(&self.conn)?;
        let sql = if trashed {
            format!(
                "SELECT {TRACK_COLUMNS} FROM tracks
                 WHERE trashed_at IS NOT NULL
                 ORDER BY trashed_at DESC, artist COLLATE NOCASE, title COLLATE NOCASE"
            )
        } else {
            format!(
                "SELECT {TRACK_COLUMNS} FROM tracks
                 WHERE trashed_at IS NULL
                 ORDER BY artist COLLATE NOCASE, title COLLATE NOCASE"
            )
        };
        let mut stmt = conn.prepare(&sql)?;
        let tracks = stmt
            .query_map([], map_track_row)?
            .collect::<Result<Vec<_>, _>>()?;
        Ok(tracks
            .into_iter()
            .map(|track| self.expand_track(track))
            .collect())
    }

    pub fn list_all_tracks(&self) -> Result<Vec<Track>, rusqlite::Error> {
        let conn = lock_conn(&self.conn)?;
        let mut stmt = conn.prepare(&format!(
            "SELECT {TRACK_COLUMNS} FROM tracks ORDER BY id"
        ))?;
        let tracks = stmt
            .query_map([], map_track_row)?
            .collect::<Result<Vec<_>, _>>()?;
        Ok(tracks
            .into_iter()
            .map(|track| self.expand_track(track))
            .collect())
    }

    pub fn get_track(&self, id: i64) -> Result<Option<Track>, rusqlite::Error> {
        let conn = lock_conn(&self.conn)?;
        let mut stmt = conn.prepare(&format!(
            "SELECT {TRACK_COLUMNS} FROM tracks WHERE id = ?1"
        ))?;
        let mut rows = stmt.query_map(params![id], map_track_row)?;
        Ok(rows.next().transpose()?.map(|track| self.expand_track(track)))
    }

    pub fn find_by_content_hash(&self, hash: &str) -> Result<Option<Track>, rusqlite::Error> {
        let conn = lock_conn(&self.conn)?;
        let mut stmt = conn.prepare(&format!(
            "SELECT {TRACK_COLUMNS} FROM tracks
             WHERE content_hash = ?1 AND trashed_at IS NULL
             LIMIT 1"
        ))?;
        let mut rows = stmt.query_map(params![hash], map_track_row)?;
        Ok(rows.next().transpose()?.map(|track| self.expand_track(track)))
    }

    pub fn find_by_stored_path(&self, relative: &str) -> Result<Option<Track>, rusqlite::Error> {
        let conn = lock_conn(&self.conn)?;
        let mut stmt = conn.prepare(&format!(
            "SELECT {TRACK_COLUMNS} FROM tracks WHERE path = ?1 LIMIT 1"
        ))?;
        let mut rows = stmt.query_map(params![relative], map_track_row)?;
        Ok(rows.next().transpose()?.map(|track| self.expand_track(track)))
    }

    pub fn find_duplicate(
        &self,
        title: Option<&str>,
        artist: Option<&str>,
        duration_ms: Option<u64>,
    ) -> Result<Option<Track>, rusqlite::Error> {
        let Some(title) = title.map(str::trim).filter(|value| !value.is_empty()) else {
            return Ok(None);
        };

        let conn = lock_conn(&self.conn)?;
        let mut stmt = conn.prepare(&format!(
            "SELECT {TRACK_COLUMNS} FROM tracks
             WHERE trashed_at IS NULL
               AND title IS NOT NULL
               AND trim(title) != ''
               AND lower(trim(title)) = lower(trim(?1))"
        ))?;

        let rows = stmt.query_map(params![title], map_track_row)?;
        for row in rows {
            let track = self.expand_track(row?);
            if is_same_track(&track, Some(title), artist, duration_ms) {
                return Ok(Some(track));
            }
        }

        Ok(None)
    }

    pub fn insert_track(&self, track: NewTrack<'_>) -> Result<i64, rusqlite::Error> {
        let conn = lock_conn(&self.conn)?;
        conn.execute(
            "INSERT INTO tracks (
                path, title, artist, album, duration_ms, bpm, bitrate_kbps,
                genre, key_name, rating, artwork_path, source, converted, added_at,
                content_hash, parent_track_id, format_group_id, rekordbox_content_id
             ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17, ?18)",
            params![
                track.path,
                track.title,
                track.artist,
                track.album,
                track.duration_ms.map(|value| value as i64),
                track.bpm,
                track.bitrate_kbps,
                track.genre,
                track.key,
                track.rating,
                track.artwork_path,
                track.source,
                track.converted as i64,
                track.added_at,
                track.content_hash,
                track.parent_track_id,
                track.format_group_id,
                track.rekordbox_content_id
            ],
        )?;
        Ok(conn.last_insert_rowid())
    }

    pub fn update_location(&self, id: i64, relative_path: &str) -> Result<(), rusqlite::Error> {
        let conn = lock_conn(&self.conn)?;
        conn.execute(
            "UPDATE tracks SET path = ?1 WHERE id = ?2",
            params![relative_path, id],
        )?;
        Ok(())
    }

    pub fn update_artwork_path(
        &self,
        id: i64,
        relative_path: Option<&str>,
    ) -> Result<(), rusqlite::Error> {
        let conn = lock_conn(&self.conn)?;
        conn.execute(
            "UPDATE tracks SET artwork_path = ?1 WHERE id = ?2",
            params![relative_path, id],
        )?;
        Ok(())
    }

    pub fn set_trashed(
        &self,
        id: i64,
        trashed_at: Option<i64>,
        relative_path: &str,
    ) -> Result<(), rusqlite::Error> {
        let conn = lock_conn(&self.conn)?;
        conn.execute(
            "UPDATE tracks SET trashed_at = ?1, path = ?2 WHERE id = ?3",
            params![trashed_at, relative_path, id],
        )?;
        Ok(())
    }

    pub fn set_rekordbox_content_id(
        &self,
        id: i64,
        content_id: Option<&str>,
    ) -> Result<(), rusqlite::Error> {
        let conn = lock_conn(&self.conn)?;
        conn.execute(
            "UPDATE tracks SET rekordbox_content_id = ?1 WHERE id = ?2",
            params![content_id, id],
        )?;
        Ok(())
    }

    pub fn set_format_group_id(
        &self,
        id: i64,
        group_id: Option<&str>,
    ) -> Result<(), rusqlite::Error> {
        let conn = lock_conn(&self.conn)?;
        conn.execute(
            "UPDATE tracks SET format_group_id = ?1 WHERE id = ?2",
            params![group_id, id],
        )?;
        Ok(())
    }

    pub fn children_of(&self, parent_id: i64) -> Result<Vec<Track>, rusqlite::Error> {
        let conn = lock_conn(&self.conn)?;
        let mut stmt = conn.prepare(&format!(
            "SELECT {TRACK_COLUMNS} FROM tracks WHERE parent_track_id = ?1"
        ))?;
        let tracks = stmt
            .query_map(params![parent_id], map_track_row)?
            .collect::<Result<Vec<_>, _>>()?;
        Ok(tracks
            .into_iter()
            .map(|track| self.expand_track(track))
            .collect())
    }

    pub fn delete_track_row(&self, id: i64) -> Result<bool, rusqlite::Error> {
        let conn = lock_conn(&self.conn)?;
        let rows = conn.execute("DELETE FROM tracks WHERE id = ?1", params![id])?;
        Ok(rows > 0)
    }
}

pub fn save_artwork_for_track(
    artwork_dir: &Path,
    track_id: i64,
    data: &[u8],
    mime: &str,
) -> Option<String> {
    let ext = match mime {
        "image/png" => "png",
        "image/gif" => "gif",
        "image/webp" => "webp",
        "image/bmp" => "bmp",
        _ => "jpg",
    };
    let file_name = format!("{track_id}.{ext}");
    let file_path = artwork_dir.join(&file_name);
    if std::fs::write(&file_path, data).is_ok() {
        Some(format!("artwork/{file_name}"))
    } else {
        None
    }
}

fn map_track_row(row: &Row<'_>) -> rusqlite::Result<Track> {
    let stored_path: String = row.get(1)?;
    let stored_artwork_path: Option<String> = row.get(11)?;
    Ok(Track {
        id: row.get(0)?,
        path: stored_path.clone(),
        title: row.get(2)?,
        artist: row.get(3)?,
        album: row.get(4)?,
        duration_ms: row.get::<_, Option<i64>>(5)?.map(|value| value as u64),
        bpm: row.get(6)?,
        bitrate_kbps: row.get(7)?,
        genre: row.get(8)?,
        key: row.get(9)?,
        rating: row.get(10)?,
        artwork_path: stored_artwork_path.clone(),
        source: row.get(12)?,
        converted: row.get::<_, i64>(13)? != 0,
        added_at: row.get(14)?,
        content_hash: row.get(15)?,
        trashed_at: row.get(16)?,
        parent_track_id: row.get(17)?,
        format_group_id: row.get(18)?,
        rekordbox_content_id: row.get(19)?,
        stored_path,
        stored_artwork_path,
    })
}

fn migrate(conn: &Connection) -> Result<(), rusqlite::Error> {
    let migrations = [
        "ALTER TABLE tracks ADD COLUMN bitrate_kbps INTEGER",
        "ALTER TABLE tracks ADD COLUMN genre TEXT",
        "ALTER TABLE tracks ADD COLUMN key_name TEXT",
        "ALTER TABLE tracks ADD COLUMN rating INTEGER",
        "ALTER TABLE tracks ADD COLUMN artwork_path TEXT",
        "ALTER TABLE tracks ADD COLUMN source TEXT",
        "ALTER TABLE tracks ADD COLUMN converted INTEGER NOT NULL DEFAULT 0",
        "ALTER TABLE tracks ADD COLUMN content_hash TEXT",
        "ALTER TABLE tracks ADD COLUMN trashed_at INTEGER",
        "ALTER TABLE tracks ADD COLUMN parent_track_id INTEGER",
        "ALTER TABLE tracks ADD COLUMN format_group_id TEXT",
        "ALTER TABLE tracks ADD COLUMN rekordbox_content_id TEXT",
    ];

    for sql in migrations {
        conn.execute(sql, []).ok();
    }

    conn.execute(
        "CREATE INDEX IF NOT EXISTS idx_tracks_content_hash ON tracks(content_hash)",
        [],
    )
    .ok();
    conn.execute(
        "CREATE INDEX IF NOT EXISTS idx_tracks_trashed_at ON tracks(trashed_at)",
        [],
    )
    .ok();
    conn.execute(
        "CREATE INDEX IF NOT EXISTS idx_tracks_parent ON tracks(parent_track_id)",
        [],
    )
    .ok();

    Ok(())
}

pub fn init_library(app: &AppHandle) -> Result<(), Box<dyn std::error::Error>> {
    let root = resolve_library_root();
    ensure_library_layout(&root)?;
    let state = LibraryState::new(root)?;
    app.manage(state);
    app.manage(crate::library::DuplicateResolver::new());
    Ok(())
}
