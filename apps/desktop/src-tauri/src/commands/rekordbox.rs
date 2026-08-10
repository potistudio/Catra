use crate::rekordbox::{
    check, db_status, get_content, get_playlist_content, list_playlists, RekordboxCheck,
    RekordboxContent, RekordboxDbStatus, RekordboxPlaylist,
};

#[tauri::command]
pub fn rekordbox_check() -> Result<RekordboxCheck, String> {
    check()
}

#[tauri::command]
pub fn rekordbox_db_status() -> Result<RekordboxDbStatus, String> {
    db_status()
}

#[tauri::command]
pub fn rekordbox_get_content(id: Option<String>) -> Result<Vec<RekordboxContent>, String> {
    get_content(id)
}

#[tauri::command]
pub fn rekordbox_list_playlists() -> Result<Vec<RekordboxPlaylist>, String> {
    list_playlists()
}

#[tauri::command]
pub fn rekordbox_get_playlist_content(
    playlist_id: String,
) -> Result<Vec<RekordboxContent>, String> {
    get_playlist_content(playlist_id)
}
