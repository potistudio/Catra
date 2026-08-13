use crate::library::{remove_rekordbox_content_id, LibraryState};
use crate::rekordbox::{
    add_content, add_to_playlist, check, create_playlist, create_playlist_folder, db_status,
    delete_playlist, get_content, get_playlist_content, list_playlists, move_song_in_playlist,
    remove_from_playlist, rename_playlist, update_content, RekordboxCheck, RekordboxContent,
    RekordboxContentUpdate, RekordboxDbStatus, RekordboxPlaylist,
};
use tauri::State;

// Blocking I/O (SQLCipher, filesystem, process list). `async` runs the handler off the
// UI main thread; non-async Tauri commands execute on the main thread.
#[tauri::command(async)]
pub fn rekordbox_check() -> Result<RekordboxCheck, String> {
    check()
}

#[tauri::command(async)]
pub fn rekordbox_db_status() -> Result<RekordboxDbStatus, String> {
    db_status()
}

#[tauri::command(async)]
pub fn rekordbox_get_content(id: Option<String>) -> Result<Vec<RekordboxContent>, String> {
    get_content(id)
}

#[tauri::command(async)]
pub fn rekordbox_list_playlists() -> Result<Vec<RekordboxPlaylist>, String> {
    list_playlists()
}

#[tauri::command(async)]
pub fn rekordbox_get_playlist_content(
    playlist_id: String,
) -> Result<Vec<RekordboxContent>, String> {
    get_playlist_content(playlist_id)
}

#[tauri::command(async)]
pub fn rekordbox_create_playlist(
    name: String,
    parent_id: Option<String>,
) -> Result<RekordboxPlaylist, String> {
    create_playlist(name, parent_id)
}

#[tauri::command(async)]
pub fn rekordbox_create_playlist_folder(
    name: String,
    parent_id: Option<String>,
) -> Result<RekordboxPlaylist, String> {
    create_playlist_folder(name, parent_id)
}

#[tauri::command(async)]
pub fn rekordbox_rename_playlist(id: String, name: String) -> Result<RekordboxPlaylist, String> {
    rename_playlist(id, name)
}

#[tauri::command(async)]
pub fn rekordbox_delete_playlist(id: String) -> Result<(), String> {
    delete_playlist(id)
}

#[tauri::command(async)]
pub fn rekordbox_add_to_playlist(
    playlist_id: String,
    content_id: String,
    track_no: Option<i32>,
) -> Result<String, String> {
    add_to_playlist(playlist_id, content_id, track_no)
}

#[tauri::command(async)]
pub fn rekordbox_remove_from_playlist(
    playlist_id: String,
    song_playlist_id: String,
) -> Result<(), String> {
    remove_from_playlist(playlist_id, song_playlist_id)
}

#[tauri::command(async)]
pub fn rekordbox_move_song_in_playlist(
    playlist_id: String,
    song_playlist_id: String,
    new_track_no: i32,
) -> Result<(), String> {
    move_song_in_playlist(playlist_id, song_playlist_id, new_track_no)
}

#[tauri::command(async)]
pub fn rekordbox_add_content(
    path: String,
    title: Option<String>,
) -> Result<RekordboxContent, String> {
    add_content(path, title)
}

#[tauri::command(async)]
pub fn rekordbox_update_content(
    id: String,
    fields: RekordboxContentUpdate,
) -> Result<RekordboxContent, String> {
    update_content(id, fields)
}

/// Delete Content from the collection. Catra keeps Cue and analysis for re-add.
#[tauri::command(async)]
pub fn rekordbox_delete_content(
    state: State<'_, LibraryState>,
    id: String,
) -> Result<(), String> {
    remove_rekordbox_content_id(&state, &id)
}
