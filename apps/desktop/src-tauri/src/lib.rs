mod activity_log;
mod commands;
mod download;
mod extension_server;
mod library;
mod rekordbox;

use activity_log::emit_activity_log;
use commands::{
    browse_facet, browse_facet_tracks, browse_tag_tracks, library_add_to_rekordbox,
    library_check_health, library_convert_tracks, library_delete_permanently, library_empty_trash,
    library_import_from_rekordbox, library_import_paths, library_list_tracks, library_list_trashed,
    library_remove_from_rekordbox, library_remove_track, library_remove_tracks,
    library_resolve_duplicate, library_restore_tracks, library_scan_folder, playlist_add_tracks,
    playlist_create, playlist_delete, playlist_delete_impact, playlist_entries, playlist_list_tree,
    playlist_move, playlist_remove_entries, playlist_remove_tracks, playlist_rename,
    playlist_reorder_entries, playlist_set_rule, rekordbox_add_content, rekordbox_add_to_playlist,
    rekordbox_check, rekordbox_create_playlist, rekordbox_create_playlist_folder,
    rekordbox_db_status, rekordbox_delete_content, rekordbox_delete_playlist,
    rekordbox_get_content, rekordbox_get_playlist_content, rekordbox_list_playlists,
    rekordbox_move_song_in_playlist, rekordbox_remove_from_playlist, rekordbox_rename_playlist,
    rekordbox_update_content, tag_assign, tag_axis_conflicts, tag_create, tag_create_axis,
    tag_delete, tag_delete_axis, tag_list_axes, tag_of_tracks, tag_rename, tag_unassign,
    tag_update_axis,
};
use extension_server::start as start_extension_server;
use library::{check_health_with, init_library, HealthDepth, LibraryState};
use tauri::{AppHandle, Manager};

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_window_state::Builder::default().build())
        .setup(|app| {
            init_library(app.handle())?;
            spawn_startup_health_check(app.handle().clone());
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
            playlist_list_tree,
            playlist_create,
            playlist_rename,
            playlist_move,
            playlist_delete_impact,
            playlist_delete,
            playlist_set_rule,
            playlist_entries,
            playlist_add_tracks,
            playlist_remove_entries,
            playlist_remove_tracks,
            playlist_reorder_entries,
            tag_list_axes,
            tag_create_axis,
            tag_update_axis,
            tag_axis_conflicts,
            tag_delete_axis,
            tag_create,
            tag_rename,
            tag_delete,
            tag_assign,
            tag_unassign,
            tag_of_tracks,
            browse_facet,
            browse_facet_tracks,
            browse_tag_tracks,
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

/// 起動時の健全性チェック。ウィンドウ表示を待たせないよう別スレッドで動かし、
/// ライブラリ全体を読み込むハッシュ検証は手動の `library_check_health` に任せる。
fn spawn_startup_health_check(app: AppHandle) {
    std::thread::spawn(move || {
        let library = app.state::<LibraryState>();
        let Ok(report) = check_health_with(&library, HealthDepth::Quick) else {
            return;
        };
        if !report.has_issues() {
            return;
        }
        emit_activity_log(
            &app,
            "warning",
            format!("ライブラリ健全性: 欠損 {}", report.missing),
            None,
        );
    });
}
