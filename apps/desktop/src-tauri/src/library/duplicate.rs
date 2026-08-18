use super::db::Track;
use serde::Serialize;
use std::sync::{Condvar, Mutex};
use tauri::{AppHandle, Emitter};

pub const DURATION_TOLERANCE_MS: u64 = 3000;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DuplicateChoice {
    KeepExisting,
    KeepNew,
    KeepAsAltFormat,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TrackCandidate {
    pub path: String,
    pub title: Option<String>,
    pub artist: Option<String>,
    pub album: Option<String>,
    pub duration_ms: Option<u64>,
    pub bpm: Option<f32>,
    pub bitrate_kbps: Option<u32>,
    pub genre: Option<String>,
    pub key: Option<String>,
    pub rating: Option<u8>,
    pub source: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DuplicateFoundPayload {
    pub existing: Track,
    pub candidate: TrackCandidate,
    pub allow_alt_format: bool,
}

pub struct DuplicateResolver {
    inner: Mutex<ResolverInner>,
    notify: Condvar,
}

struct ResolverInner {
    resolution: Option<DuplicateChoice>,
}

impl DuplicateResolver {
    pub fn new() -> Self {
        Self {
            inner: Mutex::new(ResolverInner { resolution: None }),
            notify: Condvar::new(),
        }
    }

    pub fn request_choice(
        &self,
        app: &AppHandle,
        existing: Track,
        candidate: TrackCandidate,
        allow_alt_format: bool,
    ) -> DuplicateChoice {
        let mut inner = self.inner.lock().unwrap();
        inner.resolution = None;

        let payload = DuplicateFoundPayload {
            existing,
            candidate,
            allow_alt_format,
        };
        let _ = app.emit("library-duplicate-found", &payload);

        while inner.resolution.is_none() {
            inner = self.notify.wait(inner).unwrap();
        }

        inner.resolution.unwrap()
    }

    pub fn resolve(&self, choice: DuplicateChoice) -> Result<(), String> {
        let mut inner = self.inner.lock().unwrap();
        inner.resolution = Some(choice);
        self.notify.notify_one();
        Ok(())
    }
}

pub fn normalize_field(value: &str) -> String {
    value.trim().to_ascii_lowercase()
}

pub fn is_same_track(
    existing: &Track,
    title: Option<&str>,
    artist: Option<&str>,
    duration_ms: Option<u64>,
) -> bool {
    let new_title = title.filter(|value| !value.trim().is_empty());
    let new_artist = artist.filter(|value| !value.trim().is_empty());
    let existing_title = existing
        .title
        .as_deref()
        .filter(|value| !value.trim().is_empty());
    let existing_artist = existing
        .artist
        .as_deref()
        .filter(|value| !value.trim().is_empty());

    if new_title.is_none() || existing_title.is_none() {
        return false;
    }

    if normalize_field(new_title.unwrap()) != normalize_field(existing_title.unwrap()) {
        return false;
    }

    match (existing_artist, new_artist) {
        (Some(existing_value), Some(new_value)) => {
            if normalize_field(existing_value) != normalize_field(new_value) {
                return false;
            }
        }
        (None, None) => {}
        _ => return false,
    }

    match (existing.duration_ms, duration_ms) {
        (Some(existing_duration), Some(new_duration)) => {
            let diff = if existing_duration > new_duration {
                existing_duration - new_duration
            } else {
                new_duration - existing_duration
            };
            if diff > DURATION_TOLERANCE_MS {
                return false;
            }
        }
        _ => {}
    }

    true
}
