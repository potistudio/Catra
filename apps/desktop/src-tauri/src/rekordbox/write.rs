use super::config::{is_rekordbox_running, load_config};
use super::db::MasterDatabase;
use super::playlist_xml::MasterPlaylistXml;
use super::registry::UsnBuffer;
use chrono::{DateTime, Local};
use rusqlite::Connection;
use std::path::PathBuf;

/// Reject writes while Rekordbox holds the database.
pub fn ensure_writable() -> Result<(), String> {
    if is_rekordbox_running() {
        return Err(
            "Rekordbox is running. Close Rekordbox before modifying the library.".to_string(),
        );
    }
    Ok(())
}

pub fn now_local() -> DateTime<Local> {
    Local::now()
}

/// Timestamp text stored in `created_at` / `updated_at` columns.
pub fn timestamp_sql(now: DateTime<Local>) -> String {
    now.format("%Y-%m-%d %H:%M:%S%.3f").to_string()
}

pub struct WriteSession {
    pub db: MasterDatabase,
    pub db_dir: PathBuf,
    pub playlist_xml: Option<MasterPlaylistXml>,
    pub usn: UsnBuffer,
}

impl WriteSession {
    pub fn open() -> Result<Self, String> {
        ensure_writable()?;
        let config = load_config()?;
        let db_path = config
            .db_path
            .ok_or_else(|| "Rekordbox master.db was not found".to_string())?;
        let db_dir = config
            .db_dir
            .ok_or_else(|| "Rekordbox database directory was not found".to_string())?;
        let playlist_xml = MasterPlaylistXml::open(&db_dir)?;
        Ok(Self {
            db: MasterDatabase::open_path(&db_path)?,
            db_dir,
            playlist_xml,
            usn: UsnBuffer::default(),
        })
    }

    pub fn conn(&self) -> &Connection {
        self.db.conn()
    }

    pub fn conn_and_usn(&mut self) -> (&Connection, &mut UsnBuffer) {
        (&self.db.conn, &mut self.usn)
    }

    pub fn commit(mut self) -> Result<(), String> {
        self.usn.apply(&self.db.conn)?;
        if let Some(xml) = self.playlist_xml.as_mut() {
            xml.save()?;
        }
        Ok(())
    }
}
