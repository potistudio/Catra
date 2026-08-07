mod commands;
mod library;

use commands::{library_list_tracks, library_remove_track, library_scan_folder};
use library::init_library;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_opener::init())
        .setup(|app| {
            init_library(app.handle())?;
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            library_list_tracks,
            library_scan_folder,
            library_remove_track,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
