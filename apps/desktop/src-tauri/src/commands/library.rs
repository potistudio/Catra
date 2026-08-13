use crate::library::{
    start_convert_tracks, start_import_from_rekordbox, start_scan_folder, ConvertOptions,
    DuplicateChoice, DuplicateResolver, LibraryState, Track,
};
use std::path::Path;
use tauri::{AppHandle, State};

#[tauri::command]
pub fn library_list_tracks(state: State<'_, LibraryState>) -> Result<Vec<Track>, String> {
    state.list_tracks().map_err(|e| e.to_string())
}

#[tauri::command]
pub fn library_scan_folder(app: AppHandle, folder: String) -> Result<(), String> {
    if !Path::new(&folder).is_dir() {
        return Err(format!("Not a directory: {folder}"));
    }

    start_scan_folder(app, folder);
    Ok(())
}

#[tauri::command]
pub fn library_import_from_rekordbox(app: AppHandle) -> Result<(), String> {
    start_import_from_rekordbox(app);
    Ok(())
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

#[tauri::command]
pub fn library_remove_tracks(state: State<'_, LibraryState>, ids: Vec<i64>) -> Result<u32, String> {
    state.remove_tracks(&ids).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn library_resolve_duplicate(
    resolver: State<'_, DuplicateResolver>,
    choice: String,
) -> Result<(), String> {
    let choice = match choice.as_str() {
        "existing" => DuplicateChoice::KeepExisting,
        "new" => DuplicateChoice::KeepNew,
        _ => return Err(format!("Invalid duplicate choice: {choice}")),
    };

    resolver.resolve(choice)
}

#[tauri::command]
pub fn library_convert_tracks(
    app: AppHandle,
    ids: Vec<i64>,
    options: ConvertOptions,
) -> Result<(), String> {
    start_convert_tracks(app, ids, options)
}
