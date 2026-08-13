mod config;
mod content;
mod db;
mod ids;
mod key;
mod playlist;
mod playlist_xml;
mod registry;
mod write;

pub use content::{RekordboxContent, RekordboxContentUpdate};
pub use playlist::RekordboxPlaylist;

use config::{is_rekordbox_running, load_config, RekordboxConfig};
use content::{
    add_content as add_content_impl, delete_content as delete_content_impl,
    hide_content as hide_content_impl, restore_content as restore_content_impl,
    update_content as update_content_impl,
    update_content_folder_path as update_content_folder_path_impl,
};
use write::ensure_writable as ensure_writable_impl;
use db::MasterDatabase;
use playlist::{
    add_to_playlist as add_to_playlist_impl, create_playlist as create_playlist_impl,
    create_playlist_folder as create_playlist_folder_impl, delete_playlist as delete_playlist_impl,
    move_song_in_playlist as move_song_in_playlist_impl,
    remove_from_playlist as remove_from_playlist_impl, rename_playlist as rename_playlist_impl,
};
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
    let (db, db_dir) = open_db()?;
    db.get_content(id.as_deref(), &db_dir)
}

pub fn list_playlists() -> Result<Vec<RekordboxPlaylist>, String> {
    let db = MasterDatabase::open()?;
    db.list_playlists()
}

pub fn get_playlist_content(playlist_id: String) -> Result<Vec<RekordboxContent>, String> {
    let (db, db_dir) = open_db()?;
    db.get_playlist_content(&playlist_id, &db_dir)
}

pub fn create_playlist(
    name: String,
    parent_id: Option<String>,
) -> Result<RekordboxPlaylist, String> {
    create_playlist_impl(name, parent_id)
}

pub fn create_playlist_folder(
    name: String,
    parent_id: Option<String>,
) -> Result<RekordboxPlaylist, String> {
    create_playlist_folder_impl(name, parent_id)
}

pub fn rename_playlist(id: String, name: String) -> Result<RekordboxPlaylist, String> {
    rename_playlist_impl(id, name)
}

pub fn delete_playlist(id: String) -> Result<(), String> {
    delete_playlist_impl(id)
}

pub fn add_to_playlist(
    playlist_id: String,
    content_id: String,
    track_no: Option<i32>,
) -> Result<String, String> {
    add_to_playlist_impl(playlist_id, content_id, track_no)
}

pub fn remove_from_playlist(playlist_id: String, song_playlist_id: String) -> Result<(), String> {
    remove_from_playlist_impl(playlist_id, song_playlist_id)
}

pub fn move_song_in_playlist(
    playlist_id: String,
    song_playlist_id: String,
    new_track_no: i32,
) -> Result<(), String> {
    move_song_in_playlist_impl(playlist_id, song_playlist_id, new_track_no)
}

pub fn add_content(path: String, title: Option<String>) -> Result<RekordboxContent, String> {
    add_content_impl(path, title)
}

pub fn hide_content(id: String) -> Result<(), String> {
    hide_content_impl(id)
}

pub fn restore_content(id: &str, path: Option<&str>) -> Result<RekordboxContent, String> {
    restore_content_impl(id, path)
}

pub fn update_content(
    id: String,
    fields: RekordboxContentUpdate,
) -> Result<RekordboxContent, String> {
    update_content_impl(id, fields)
}

pub fn delete_content(id: String) -> Result<(), String> {
    delete_content_impl(id)
}

pub fn update_content_folder_path(id: &str, path: &str) -> Result<RekordboxContent, String> {
    update_content_folder_path_impl(id, path)
}

pub fn ensure_writable() -> Result<(), String> {
    ensure_writable_impl()
}

fn open_db() -> Result<(MasterDatabase, std::path::PathBuf), String> {
    let config = load_config()?;
    let db_path = config
        .db_path
        .ok_or_else(|| "Rekordbox master.db was not found".to_string())?;
    let db_dir = config
        .db_dir
        .ok_or_else(|| "Rekordbox database directory was not found".to_string())?;
    Ok((MasterDatabase::open_path(&db_path)?, db_dir))
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
