use rusqlite::{params, Connection};
use serde::Serialize;
use std::path::PathBuf;
use std::sync::Mutex;
use tauri::{AppHandle, Manager};

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
}

impl LibraryState {
    pub fn new(db_path: PathBuf) -> Result<Self, rusqlite::Error> {
        if let Some(parent) = db_path.parent() {
            std::fs::create_dir_all(parent).ok();
        }

        let conn = Connection::open(db_path)?;
        conn.execute_batch(
            "
            CREATE TABLE IF NOT EXISTS tracks (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                path TEXT NOT NULL UNIQUE,
                title TEXT,
                artist TEXT,
                album TEXT,
                duration_ms INTEGER,
                bpm REAL,
                added_at INTEGER NOT NULL
            );
            CREATE INDEX IF NOT EXISTS idx_tracks_artist ON tracks(artist);
            CREATE INDEX IF NOT EXISTS idx_tracks_title ON tracks(title);
            ",
        )?;

        Ok(Self {
            conn: Mutex::new(conn),
        })
    }

    pub fn list_tracks(&self) -> Result<Vec<Track>, rusqlite::Error> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT id, path, title, artist, album, duration_ms, bpm, added_at
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
                    added_at: row.get(7)?,
                })
            })?
            .collect::<Result<Vec<_>, _>>()?;

        Ok(tracks)
    }

    pub fn insert_track(
        &self,
        path: &str,
        title: Option<&str>,
        artist: Option<&str>,
        album: Option<&str>,
        duration_ms: Option<u64>,
        bpm: Option<f32>,
        added_at: i64,
    ) -> Result<bool, rusqlite::Error> {
        let conn = self.conn.lock().unwrap();
        let rows = conn.execute(
            "INSERT OR IGNORE INTO tracks (path, title, artist, album, duration_ms, bpm, added_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            params![path, title, artist, album, duration_ms, bpm, added_at],
        )?;

        Ok(rows > 0)
    }

    pub fn remove_track(&self, id: i64) -> Result<bool, rusqlite::Error> {
        let conn = self.conn.lock().unwrap();
        let rows = conn.execute("DELETE FROM tracks WHERE id = ?1", params![id])?;
        Ok(rows > 0)
    }
}

pub fn init_library(app: &AppHandle) -> Result<(), Box<dyn std::error::Error>> {
    let db_path = app
        .path()
        .app_data_dir()?
        .join("library.db");

    let state = LibraryState::new(db_path)?;
    app.manage(state);

    Ok(())
}
