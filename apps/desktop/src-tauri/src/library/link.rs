use super::db::LibraryState;

pub fn add_to_rekordbox(library: &LibraryState, id: i64) -> Result<(), String> {
    let track = library
        .get_track(id)
        .map_err(|error| error.to_string())?
        .ok_or_else(|| format!("Track not found: {id}"))?;
    if track.trashed_at.is_some() {
        return Err("ゴミ箱のトラックは Rekordbox に追加できません".to_string());
    }
    if let Some(snapshot) = library
        .rekordbox_snapshot(id)
        .map_err(|error| error.to_string())?
    {
        match crate::rekordbox::import_content_snapshot(&snapshot, Some(&track.path)) {
            Ok(content) => {
                library
                    .set_rekordbox_content_id(id, Some(&content.id))
                    .map_err(|error| error.to_string())?;
                library
                    .set_rekordbox_snapshot(id, None)
                    .map_err(|error| error.to_string())?;
                return Ok(());
            }
            Err(error) if error.contains("was not found") => {}
            Err(error) => return Err(error),
        }
    }
    if let Some(content_id) = track.rekordbox_content_id.as_deref() {
        match crate::rekordbox::restore_content(content_id, Some(&track.path)) {
            Ok(_) => {
                library
                    .set_rekordbox_snapshot(id, None)
                    .map_err(|error| error.to_string())?;
                return Ok(());
            }
            Err(error) if error.contains("was not found") => {}
            Err(error) => return Err(error),
        }
    }
    let content = crate::rekordbox::add_content(track.path.clone(), track.title.clone())?;
    library
        .set_rekordbox_content_id(id, Some(&content.id))
        .map_err(|error| error.to_string())?;
    library
        .set_rekordbox_snapshot(id, None)
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
    unlink_content(library, id, &content_id)
}

pub fn remove_content_id(library: &LibraryState, content_id: &str) -> Result<(), String> {
    if let Some(track_id) = library
        .find_id_by_rekordbox_content_id(content_id)
        .map_err(|error| error.to_string())?
    {
        remove_from_rekordbox(library, track_id).map(|_| ())
    } else {
        crate::rekordbox::delete_content(content_id.to_string())
    }
}

fn unlink_content(library: &LibraryState, id: i64, content_id: &str) -> Result<bool, String> {
    let snapshot = match crate::rekordbox::export_content_snapshot(content_id) {
        Ok(snapshot) => Some(snapshot),
        Err(error) if error.contains("was not found") => None,
        Err(error) => return Err(error),
    };
    match crate::rekordbox::delete_content(content_id.to_string()) {
        Err(error) if error.contains("was not found") => {
            if snapshot.is_none() {
                library
                    .set_rekordbox_content_id(id, None)
                    .map_err(|error| error.to_string())?;
                library
                    .set_rekordbox_snapshot(id, None)
                    .map_err(|error| error.to_string())?;
                return Ok(false);
            }
        }
        Err(error) => return Err(error),
        Ok(()) => {}
    }
    if let Some(snapshot) = snapshot.as_deref() {
        library
            .set_rekordbox_snapshot(id, Some(snapshot))
            .map_err(|error| error.to_string())?;
    }
    Ok(true)
}
