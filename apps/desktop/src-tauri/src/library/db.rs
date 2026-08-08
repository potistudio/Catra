use super::duplicate::is_same_track;
use rusqlite::{params, Connection};
use serde::Serialize;
use std::hash::{Hash, Hasher};
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
    pub added_at: i64,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ScanResult {
    pub added: u32,
    pub skipped: u32,
}

pub struct LibraryState {
    conn: Mutex<Connection>,
    artwork_dir: PathBuf,
}

impl LibraryState {
    pub fn new(db_path: PathBuf, artwork_dir: PathBuf) -> Result<Self, rusqlite::Error> {
        if let Some(parent) = db_path.parent() {
            std::fs::create_dir_all(parent).ok();
        }
        std::fs::create_dir_all(&artwork_dir).ok();

        let conn = Connection::open(db_path)?;
        conn.busy_timeout(std::time::Duration::from_secs(5))?;
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
                added_at INTEGER NOT NULL
            );
            CREATE INDEX IF NOT EXISTS idx_tracks_artist ON tracks(artist);
            CREATE INDEX IF NOT EXISTS idx_tracks_title ON tracks(title);
            ",
        )?;
        migrate(&conn)?;

        Ok(Self {
            conn: Mutex::new(conn),
            artwork_dir,
        })
    }

    pub fn artwork_dir(&self) -> &Path {
        &self.artwork_dir
    }

    pub fn list_tracks(&self) -> Result<Vec<Track>, rusqlite::Error> {
        let conn = lock_conn(&self.conn)?;
        let mut stmt = conn.prepare(
            "SELECT id, path, title, artist, album, duration_ms, bpm, bitrate_kbps,
                    genre, key_name, rating, artwork_path, source, added_at
             FROM tracks
             ORDER BY artist COLLATE NOCASE, title COLLATE NOCASE",
        )?;

        let tracks = stmt
            .query_map([], |row| {
                Ok(Track {
                    id: row.get(0)?,
                    path: row.get(1)?,
                    title: row.get(2)?,
                    artist: row.get(3)?,
                    album: row.get(4)?,
                    duration_ms: row.get(5)?,
                    bpm: row.get(6)?,
                    bitrate_kbps: row.get(7)?,
                    genre: row.get(8)?,
                    key: row.get(9)?,
                    rating: row.get(10)?,
                    artwork_path: row.get(11)?,
                    source: row.get(12)?,
                    added_at: row.get(13)?,
                })
            })?
            .collect::<Result<Vec<_>, _>>()?;

        Ok(tracks)
    }

    pub fn track_exists(&self, path: &str) -> Result<bool, rusqlite::Error> {
        let conn = lock_conn(&self.conn)?;
        let count = conn.query_row(
            "SELECT COUNT(*) FROM tracks WHERE path = ?1",
            params![path],
            |row| row.get::<_, i64>(0),
        )?;
        Ok(count > 0)
    }

    pub fn find_duplicate(
        &self,
        path: &str,
        title: Option<&str>,
        artist: Option<&str>,
        duration_ms: Option<u64>,
    ) -> Result<Option<Track>, rusqlite::Error> {
        let tracks = self.list_tracks()?;
        Ok(tracks
            .into_iter()
            .find(|track| track.path != path && is_same_track(track, title, artist, duration_ms)))
    }

    pub fn insert_track(
        &self,
        path: &str,
        title: Option<&str>,
        artist: Option<&str>,
        album: Option<&str>,
        duration_ms: Option<u64>,
        bpm: Option<f32>,
        bitrate_kbps: Option<u32>,
        genre: Option<&str>,
        key: Option<&str>,
        rating: Option<u8>,
        artwork_path: Option<&str>,
        source: Option<&str>,
        added_at: i64,
    ) -> Result<bool, rusqlite::Error> {
        let conn = lock_conn(&self.conn)?;
        let rows = conn.execute(
            "INSERT OR IGNORE INTO tracks (
                path, title, artist, album, duration_ms, bpm, bitrate_kbps,
                genre, key_name, rating, artwork_path, source, added_at
             ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13)",
            params![
                path,
                title,
                artist,
                album,
                duration_ms,
                bpm,
                bitrate_kbps,
                genre,
                key,
                rating,
                artwork_path,
                source,
                added_at
            ],
        )?;

        Ok(rows > 0)
    }

    pub fn remove_track(&self, id: i64) -> Result<bool, rusqlite::Error> {
        Ok(self.remove_tracks(&[id])? > 0)
    }

    pub fn remove_tracks(&self, ids: &[i64]) -> Result<u32, rusqlite::Error> {
        if ids.is_empty() {
            return Ok(0);
        }

        let conn = lock_conn(&self.conn)?;
        let tx = conn.unchecked_transaction()?;

        let mut artwork_paths = Vec::new();
        let mut deleted = 0u32;

        for &id in ids {
            let artwork_path: Option<String> = tx
                .query_row(
                    "SELECT artwork_path FROM tracks WHERE id = ?1",
                    params![id],
                    |row| row.get(0),
                )
                .ok();

            let rows = tx.execute("DELETE FROM tracks WHERE id = ?1", params![id])?;
            if rows > 0 {
                deleted += rows as u32;
                if let Some(path) = artwork_path {
                    artwork_paths.push(path);
                }
            }
        }

        tx.commit()?;

        if !artwork_paths.is_empty() {
            std::thread::spawn(move || {
                for path in artwork_paths {
                    std::fs::remove_file(path).ok();
                }
            });
        }

        Ok(deleted)
    }
}

pub fn save_artwork(
    artwork_dir: &Path,
    source_path: &str,
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

    let hash = path_hash(source_path);
    let file_path = artwork_dir.join(format!("{hash}.{ext}"));

    if std::fs::write(&file_path, data).is_ok() {
        Some(file_path.to_string_lossy().to_string())
    } else {
        None
    }
}

fn path_hash(path: &str) -> String {
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    path.hash(&mut hasher);
    format!("{:016x}", hasher.finish())
}

fn migrate(conn: &Connection) -> Result<(), rusqlite::Error> {
    let migrations = [
        "ALTER TABLE tracks ADD COLUMN bitrate_kbps INTEGER",
        "ALTER TABLE tracks ADD COLUMN genre TEXT",
        "ALTER TABLE tracks ADD COLUMN key_name TEXT",
        "ALTER TABLE tracks ADD COLUMN rating INTEGER",
        "ALTER TABLE tracks ADD COLUMN artwork_path TEXT",
        "ALTER TABLE tracks ADD COLUMN source TEXT",
    ];

    for sql in migrations {
        conn.execute(sql, []).ok();
    }

    Ok(())
}

pub fn init_library(app: &AppHandle) -> Result<(), Box<dyn std::error::Error>> {
    let data_dir = app.path().app_data_dir()?;
    let db_path = data_dir.join("library.db");
    let artwork_dir = data_dir.join("artwork");

    let state = LibraryState::new(db_path, artwork_dir)?;
    app.manage(state);
    app.manage(crate::library::DuplicateResolver::new());

    Ok(())
}
