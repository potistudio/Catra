use crate::library::{
    assign_tags, axis_conflicts, create_axis, create_tag, delete_axis, delete_tag, list_axes,
    rename_tag, tagged_track_ids, tags_of_tracks, unassign_tags, update_axis, LibraryState, Tag,
    TagAxis, TagSelection, Track, TrackTags,
};
use tauri::State;

#[tauri::command]
pub fn tag_list_axes(state: State<'_, LibraryState>) -> Result<Vec<TagAxis>, String> {
    list_axes(&state)
}

#[tauri::command]
pub fn tag_create_axis(
    state: State<'_, LibraryState>,
    name: String,
    selection: TagSelection,
) -> Result<TagAxis, String> {
    create_axis(&state, &name, selection)
}

#[tauri::command]
pub fn tag_update_axis(
    state: State<'_, LibraryState>,
    id: i64,
    name: Option<String>,
    selection: Option<TagSelection>,
) -> Result<TagAxis, String> {
    update_axis(&state, id, name.as_deref(), selection)
}

/// multi から single に落とす前に、はみ出す曲が何曲あるかを訊く。
#[tauri::command]
pub fn tag_axis_conflicts(state: State<'_, LibraryState>, id: i64) -> Result<u32, String> {
    axis_conflicts(&state, id)
}

#[tauri::command]
pub fn tag_delete_axis(state: State<'_, LibraryState>, id: i64) -> Result<u32, String> {
    delete_axis(&state, id)
}

#[tauri::command]
pub fn tag_create(
    state: State<'_, LibraryState>,
    axis_id: i64,
    name: String,
) -> Result<Tag, String> {
    create_tag(&state, axis_id, &name)
}

#[tauri::command]
pub fn tag_rename(state: State<'_, LibraryState>, id: i64, name: String) -> Result<Tag, String> {
    rename_tag(&state, id, &name)
}

#[tauri::command]
pub fn tag_delete(state: State<'_, LibraryState>, id: i64) -> Result<(), String> {
    delete_tag(&state, id)
}

#[tauri::command]
pub fn tag_assign(
    state: State<'_, LibraryState>,
    track_ids: Vec<i64>,
    tag_ids: Vec<i64>,
) -> Result<(), String> {
    assign_tags(&state, &track_ids, &tag_ids)
}

#[tauri::command]
pub fn tag_unassign(
    state: State<'_, LibraryState>,
    track_ids: Vec<i64>,
    tag_ids: Vec<i64>,
) -> Result<u32, String> {
    unassign_tags(&state, &track_ids, &tag_ids)
}

#[tauri::command]
pub fn tag_of_tracks(
    state: State<'_, LibraryState>,
    track_ids: Vec<i64>,
) -> Result<Vec<TrackTags>, String> {
    tags_of_tracks(&state, &track_ids)
}

/// タグでの絞り込み。複数のタグは AND で効く。その場の結果なので保存しない。
#[tauri::command]
pub fn browse_tag_tracks(
    state: State<'_, LibraryState>,
    tag_ids: Vec<i64>,
) -> Result<Vec<Track>, String> {
    let ids = tagged_track_ids(&state, &tag_ids)?;
    let mut tracks = Vec::with_capacity(ids.len());
    for id in ids {
        if let Some(track) = state.get_track(id).map_err(|error| error.to_string())? {
            tracks.push(track);
        }
    }
    Ok(tracks)
}
