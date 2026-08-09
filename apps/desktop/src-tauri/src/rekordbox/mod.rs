mod config;
mod content;
mod db;
mod key;

pub use content::RekordboxContent;

use config::{is_rekordbox_running, load_config, RekordboxConfig};
use db::MasterDatabase;
use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RekordboxCheck {
    pub db_path: Option<String>,
    pub analysis_root: Option<String>,
    pub settings_root: Option<String>,
    pub install_dir: Option<String>,
    pub version: Option<String>,
    pub rekordbox_running: bool,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RekordboxDbStatus {
    pub track_count: usize,
    pub playlist_count: usize,
}

pub fn check() -> Result<RekordboxCheck, String> {
    let config = load_config()?;
    Ok(to_check(config))
}

pub fn db_status() -> Result<RekordboxDbStatus, String> {
    let db = MasterDatabase::open()?;
    Ok(RekordboxDbStatus {
        track_count: db.track_count()?,
        playlist_count: db.playlist_count()?,
    })
}

pub fn get_content(id: Option<String>) -> Result<Vec<RekordboxContent>, String> {
    let db = MasterDatabase::open()?;
    db.get_content(id.as_deref())
}

fn to_check(config: RekordboxConfig) -> RekordboxCheck {
    RekordboxCheck {
        db_path: config.db_path.map(path_to_string),
        analysis_root: config.analysis_root.map(path_to_string),
        settings_root: config.settings_root.map(path_to_string),
        install_dir: config.install_dir.map(path_to_string),
        version: config.version,
        rekordbox_running: is_rekordbox_running(),
    }
}

fn path_to_string(path: std::path::PathBuf) -> String {
    path.to_string_lossy().into_owned()
}
