use crate::rekordbox::{
    check, db_status, get_content, RekordboxCheck, RekordboxContent, RekordboxDbStatus,
};

#[tauri::command]
pub fn rekordbox_check() -> Result<RekordboxCheck, String> {
    check()
}

#[tauri::command]
pub fn rekordbox_db_status() -> Result<RekordboxDbStatus, String> {
    db_status()
}

#[tauri::command]
pub fn rekordbox_get_content(id: Option<String>) -> Result<Vec<RekordboxContent>, String> {
    get_content(id)
}
