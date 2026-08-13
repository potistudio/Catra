use super::db::LibraryState;

pub fn add_to_rekordbox(library: &LibraryState, id: i64) -> Result<(), String> {
    let track = library
        .get_track(id)
        .map_err(|error| error.to_string())?
        .ok_or_else(|| format!("Track not found: {id}"))?;
    if track.trashed_at.is_some() {
        return Err("ゴミ箱のトラックは Rekordbox に追加できません".to_string());
    }
    let content = crate::rekordbox::add_content(track.path.clone(), track.title.clone())?;
    library
        .set_rekordbox_content_id(id, Some(&content.id))
        .map_err(|error| error.to_string())?;
    Ok(())
}

pub fn remove_from_rekordbox(library: &LibraryState, id: i64) -> Result<bool, String> {
    let track = library
        .get_track(id)
        .map_err(|error| error.to_string())?
        .ok_or_else(|| format!("Track not found: {id}"))?;
    let Some(content_id) = track.rekordbox_content_id else {
        return Ok(false);
    };
    match crate::rekordbox::delete_content(content_id) {
        Err(error) if error.contains("was not found") => {}
        Err(error) => return Err(error),
        Ok(()) => {}
    }
    library
        .set_rekordbox_content_id(id, None)
        .map_err(|error| error.to_string())?;
    Ok(true)
}
