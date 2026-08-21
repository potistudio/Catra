use crate::library::{
    add_tracks_to_playlist, create_playlist, delete_playlist, entries_of_playlist, facet_track_ids,
    facet_values, list_playlist_tree, move_playlist, playlist_deletion_impact,
    remove_playlist_entries, remove_playlist_tracks, rename_playlist, reorder_playlist_entries,
    set_playlist_rule, DeleteImpact, FacetValue, LibraryState, PlaylistEntry, PlaylistKind,
    PlaylistNode, Track,
};
use serde_json::Value;
use tauri::State;

#[tauri::command]
pub fn playlist_list_tree(state: State<'_, LibraryState>) -> Result<Vec<PlaylistNode>, String> {
    list_playlist_tree(&state)
}

#[tauri::command]
pub fn playlist_create(
    state: State<'_, LibraryState>,
    name: String,
    parent_id: Option<i64>,
    kind: PlaylistKind,
) -> Result<PlaylistNode, String> {
    create_playlist(&state, &name, parent_id, kind)
}

#[tauri::command]
pub fn playlist_rename(
    state: State<'_, LibraryState>,
    id: i64,
    name: String,
) -> Result<PlaylistNode, String> {
    rename_playlist(&state, id, &name)
}

#[tauri::command]
pub fn playlist_move(
    state: State<'_, LibraryState>,
    id: i64,
    parent_id: Option<i64>,
    position: i64,
) -> Result<(), String> {
    move_playlist(&state, id, parent_id, position)
}

#[tauri::command]
pub fn playlist_delete_impact(
    state: State<'_, LibraryState>,
    id: i64,
) -> Result<DeleteImpact, String> {
    playlist_deletion_impact(&state, id)
}

#[tauri::command]
pub fn playlist_delete(state: State<'_, LibraryState>, id: i64) -> Result<u32, String> {
    delete_playlist(&state, id)
}

#[tauri::command]
pub fn playlist_set_rule(
    state: State<'_, LibraryState>,
    id: i64,
    rule: Option<Value>,
    sort_rule: Option<Value>,
) -> Result<PlaylistNode, String> {
    set_playlist_rule(&state, id, rule, sort_rule)
}

#[tauri::command]
pub fn playlist_entries(
    state: State<'_, LibraryState>,
    id: i64,
) -> Result<Vec<PlaylistEntry>, String> {
    entries_of_playlist(&state, id)
}

#[tauri::command]
pub fn playlist_add_tracks(
    state: State<'_, LibraryState>,
    playlist_id: i64,
    track_ids: Vec<i64>,
    position: Option<i64>,
) -> Result<Vec<i64>, String> {
    add_tracks_to_playlist(&state, playlist_id, &track_ids, position)
}

#[tauri::command]
pub fn playlist_remove_entries(
    state: State<'_, LibraryState>,
    playlist_id: i64,
    entry_ids: Vec<i64>,
) -> Result<u32, String> {
    remove_playlist_entries(&state, playlist_id, &entry_ids)
}

#[tauri::command]
pub fn playlist_remove_tracks(
    state: State<'_, LibraryState>,
    playlist_id: i64,
    track_ids: Vec<i64>,
) -> Result<u32, String> {
    remove_playlist_tracks(&state, playlist_id, &track_ids)
}

#[tauri::command]
pub fn playlist_reorder_entries(
    state: State<'_, LibraryState>,
    playlist_id: i64,
    entry_ids: Vec<i64>,
) -> Result<(), String> {
    reorder_playlist_entries(&state, playlist_id, &entry_ids)
}

#[tauri::command]
pub fn browse_facet(
    state: State<'_, LibraryState>,
    field: String,
) -> Result<Vec<FacetValue>, String> {
    facet_values(&state, &field)
}

#[tauri::command]
pub fn browse_facet_tracks(
    state: State<'_, LibraryState>,
    field: String,
    value: Option<String>,
) -> Result<Vec<Track>, String> {
    let ids = facet_track_ids(&state, &field, value.as_deref())?;
    let mut tracks = Vec::with_capacity(ids.len());
    for id in ids {
        if let Some(track) = state.get_track(id).map_err(|error| error.to_string())? {
            tracks.push(track);
        }
    }
    Ok(tracks)
}
