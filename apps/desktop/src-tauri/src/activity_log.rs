use serde::Serialize;
use tauri::{AppHandle, Emitter};

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ActivityLogPayload {
    pub level: String,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub detail: Option<String>,
}

pub fn emit_activity_log(
    app: &AppHandle,
    level: &str,
    message: impl Into<String>,
    detail: Option<String>,
) {
    let payload = ActivityLogPayload {
        level: level.to_string(),
        message: message.into(),
        detail,
    };
    let _ = app.emit("activity-log", payload);
}
