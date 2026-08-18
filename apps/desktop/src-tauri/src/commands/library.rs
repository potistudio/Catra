use crate::library::{
    add_track_to_rekordbox, check_health, delete_permanently, empty_trash,
    remove_track_from_rekordbox, restore_tracks, start_convert_tracks, start_import_from_rekordbox,
    start_import_paths, start_scan_folder, trash_tracks, ConvertOptions, DuplicateChoice,
    DuplicateResolver, HealthReport, LibraryState, Track,
};
use std::path::Path;
use tauri::{AppHandle, State};

#[tauri::command]
pub fn library_list_tracks(state: State<'_, LibraryState>) -> Result<Vec<Track>, String> {
    state.list_tracks().map_err(|e| e.to_string())
}

#[tauri::command]
pub fn library_list_trashed(state: State<'_, LibraryState>) -> Result<Vec<Track>, String> {
    state.list_trashed().map_err(|e| e.to_string())
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
pub fn library_import_paths(app: AppHandle, paths: Vec<String>) -> Result<(), String> {
    if paths.is_empty() {
        return Err("インポートするパスがありません".to_string());
    }

    start_import_paths(app, paths);
    Ok(())
}

#[tauri::command]
pub fn library_import_from_rekordbox(app: AppHandle) -> Result<(), String> {
    start_import_from_rekordbox(app);
    Ok(())
}

#[tauri::command]
pub fn library_remove_track(state: State<'_, LibraryState>, id: i64) -> Result<(), String> {
    let removed = trash_tracks(&state, &[id])?;
    if removed > 0 {
        Ok(())
    } else {
        Err(format!("Track not found: {id}"))
    }
}

#[tauri::command]
pub fn library_remove_tracks(state: State<'_, LibraryState>, ids: Vec<i64>) -> Result<u32, String> {
    trash_tracks(&state, &ids)
}

#[tauri::command]
pub fn library_restore_tracks(state: State<'_, LibraryState>, ids: Vec<i64>) -> Result<u32, String> {
    restore_tracks(&state, &ids)
}

#[tauri::command]
pub fn library_delete_permanently(
    state: State<'_, LibraryState>,
    ids: Vec<i64>,
) -> Result<u32, String> {
    delete_permanently(&state, &ids)
}

#[tauri::command]
pub fn library_empty_trash(state: State<'_, LibraryState>) -> Result<u32, String> {
    empty_trash(&state)
}

#[tauri::command]
pub fn library_check_health(state: State<'_, LibraryState>) -> Result<HealthReport, String> {
    check_health(&state)
}

#[tauri::command]
pub fn library_add_to_rekordbox(state: State<'_, LibraryState>, id: i64) -> Result<(), String> {
    add_track_to_rekordbox(&state, id)
}

#[tauri::command]
pub fn library_remove_from_rekordbox(
    state: State<'_, LibraryState>,
    id: i64,
) -> Result<bool, String> {
    remove_track_from_rekordbox(&state, id)
}

#[tauri::command]
pub fn library_resolve_duplicate(
    resolver: State<'_, DuplicateResolver>,
    choice: String,
) -> Result<(), String> {
    let choice = match choice.as_str() {
        "existing" => DuplicateChoice::KeepExisting,
        "new" => DuplicateChoice::KeepNew,
        "altFormat" => DuplicateChoice::KeepAsAltFormat,
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
