mod activity_log;
mod commands;
mod download;
mod extension_server;
mod library;
mod rekordbox;

use commands::{
    library_import_from_rekordbox, library_list_tracks, library_remove_track, library_remove_tracks,
    library_resolve_duplicate, library_scan_folder, rekordbox_add_content,
    rekordbox_add_to_playlist, rekordbox_check, rekordbox_create_playlist,
    rekordbox_create_playlist_folder, rekordbox_db_status, rekordbox_delete_content,
    rekordbox_delete_playlist, rekordbox_get_content, rekordbox_get_playlist_content,
    rekordbox_list_playlists, rekordbox_move_song_in_playlist, rekordbox_remove_from_playlist,
    rekordbox_rename_playlist, rekordbox_update_content,
};
use extension_server::start as start_extension_server;
use library::init_library;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_opener::init())
        .setup(|app| {
            init_library(app.handle())?;
            start_extension_server(app.handle().clone());
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            library_list_tracks,
            library_scan_folder,
            library_import_from_rekordbox,
            library_remove_track,
            library_remove_tracks,
            library_resolve_duplicate,
            rekordbox_check,
            rekordbox_db_status,
            rekordbox_get_content,
            rekordbox_list_playlists,
            rekordbox_get_playlist_content,
            rekordbox_create_playlist,
            rekordbox_create_playlist_folder,
            rekordbox_rename_playlist,
            rekordbox_delete_playlist,
            rekordbox_add_to_playlist,
            rekordbox_remove_from_playlist,
            rekordbox_move_song_in_playlist,
            rekordbox_add_content,
            rekordbox_update_content,
            rekordbox_delete_content,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
