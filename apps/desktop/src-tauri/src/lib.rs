mod activity_log;
mod commands;
mod download;
mod extension_server;
mod library;
mod rekordbox;

use activity_log::emit_activity_log;
use commands::{
    library_add_to_rekordbox, library_check_health, library_convert_tracks,
    library_delete_permanently, library_empty_trash, library_import_from_rekordbox,
    library_import_paths, library_list_tracks, library_list_trashed, library_remove_from_rekordbox,
    library_remove_track, library_remove_tracks, library_resolve_duplicate, library_restore_tracks,
    library_scan_folder, rekordbox_add_content, rekordbox_add_to_playlist, rekordbox_check,
    rekordbox_create_playlist, rekordbox_create_playlist_folder, rekordbox_db_status,
    rekordbox_delete_content, rekordbox_delete_playlist, rekordbox_get_content,
    rekordbox_get_playlist_content, rekordbox_list_playlists, rekordbox_move_song_in_playlist,
    rekordbox_remove_from_playlist, rekordbox_rename_playlist, rekordbox_update_content,
};
use extension_server::start as start_extension_server;
use library::{check_health, init_library, LibraryState};
use tauri::Manager;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_window_state::Builder::default().build())
        .setup(|app| {
            init_library(app.handle())?;
            let library = app.state::<LibraryState>();
            if let Ok(report) = check_health(&library) {
                if report.has_issues() {
                    emit_activity_log(
                        app.handle(),
                        "warning",
                        format!(
                            "ライブラリ健全性: 欠損 {} · ハッシュ不一致 {}",
                            report.missing, report.hash_mismatch
                        ),
                        None,
                    );
                }
            }
            start_extension_server(app.handle().clone());
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            library_list_tracks,
            library_list_trashed,
            library_scan_folder,
            library_import_from_rekordbox,
            library_import_paths,
            library_remove_track,
            library_remove_tracks,
            library_restore_tracks,
            library_delete_permanently,
            library_empty_trash,
            library_check_health,
            library_add_to_rekordbox,
            library_remove_from_rekordbox,
            library_resolve_duplicate,
            library_convert_tracks,
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
