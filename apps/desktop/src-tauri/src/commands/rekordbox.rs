use crate::rekordbox::{check, db_status, RekordboxCheck, RekordboxDbStatus};

#[tauri::command]
pub fn rekordbox_check() -> Result<RekordboxCheck, String> {
    check()
}

#[tauri::command]
pub fn rekordbox_db_status() -> Result<RekordboxDbStatus, String> {
    db_status()
}
