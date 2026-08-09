use super::config::load_config;
use super::key::master_db_key;
use rusqlite::Connection;
use std::path::Path;

pub struct MasterDatabase {
    conn: Connection,
}

impl MasterDatabase {
    pub fn open() -> Result<Self, String> {
        let config = load_config()?;
        let db_path = config
            .db_path
            .ok_or_else(|| "Rekordbox master.db was not found".to_string())?;
        Self::open_path(&db_path)
    }

    pub fn open_path(path: &Path) -> Result<Self, String> {
        let key = master_db_key()?;
        let conn = Connection::open(path).map_err(|error| error.to_string())?;
        conn.pragma_update(None, "key", key)
            .map_err(|error| format!("failed to unlock Rekordbox database: {error}"))?;
        Ok(Self { conn })
    }

    pub fn track_count(&self) -> Result<usize, String> {
        self.count("SELECT COUNT(*) FROM djmdContent")
    }

    pub fn playlist_count(&self) -> Result<usize, String> {
        self.count("SELECT COUNT(*) FROM djmdPlaylist")
    }

    fn count(&self, sql: &str) -> Result<usize, String> {
        self.conn
            .query_row(sql, [], |row| row.get::<_, i64>(0))
            .map(|count| count.max(0) as usize)
            .map_err(|error| error.to_string())
    }
}
