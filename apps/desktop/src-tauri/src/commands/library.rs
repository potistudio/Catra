use crate::library::{scan_folder, LibraryState, ScanResult, Track};
use tauri::State;

#[tauri::command]
pub fn library_list_tracks(state: State<'_, LibraryState>) -> Result<Vec<Track>, String> {
    state.list_tracks().map_err(|e| e.to_string())
}

#[tauri::command]
pub fn library_scan_folder(
    state: State<'_, LibraryState>,
    folder: String,
) -> Result<ScanResult, String> {
    scan_folder(&state, &folder)
}

#[tauri::command]
pub fn library_remove_track(state: State<'_, LibraryState>, id: i64) -> Result<(), String> {
    let removed = state.remove_track(id).map_err(|e| e.to_string())?;
    if removed {
        Ok(())
    } else {
        Err(format!("Track not found: {id}"))
    }
}
