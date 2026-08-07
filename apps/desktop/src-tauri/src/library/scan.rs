use super::db::{LibraryState, ScanResult};
use lofty::file::AudioFile;
use lofty::file::TaggedFileExt;
use lofty::probe::Probe;
use lofty::tag::Accessor;
use std::path::Path;
use walkdir::WalkDir;

const AUDIO_EXTENSIONS: &[&str] = &["mp3", "flac", "wav", "aiff", "aif", "m4a", "ogg", "opus", "wma"];

pub fn scan_folder(library: &LibraryState, folder: &str) -> Result<ScanResult, String> {
    let folder_path = Path::new(folder);
    if !folder_path.is_dir() {
        return Err(format!("Not a directory: {folder}"));
    }

    let added_at = chrono::Utc::now().timestamp();
    let mut added = 0u32;
    let mut skipped = 0u32;

    for entry in WalkDir::new(folder_path)
        .follow_links(false)
        .into_iter()
        .filter_map(|e| e.ok())
    {
        let path = entry.path();
        if !path.is_file() {
            continue;
        }

        if !is_audio_file(path) {
            continue;
        }

        let path_str = path.to_string_lossy().to_string();
        let metadata = read_metadata(path);

        let inserted = library
            .insert_track(
                &path_str,
                metadata.title.as_deref(),
                metadata.artist.as_deref(),
                metadata.album.as_deref(),
                metadata.duration_ms,
                metadata.bpm,
                added_at,
            )
            .map_err(|e| e.to_string())?;

        if inserted {
            added += 1;
        } else {
            skipped += 1;
        }
    }

    Ok(ScanResult { added, skipped })
}

fn is_audio_file(path: &Path) -> bool {
    path.extension()
        .and_then(|ext| ext.to_str())
        .map(|ext| AUDIO_EXTENSIONS.contains(&ext.to_ascii_lowercase().as_str()))
        .unwrap_or(false)
}

struct FileMetadata {
    title: Option<String>,
    artist: Option<String>,
    album: Option<String>,
    duration_ms: Option<u64>,
    bpm: Option<f32>,
}

fn read_metadata(path: &Path) -> FileMetadata {
    let fallback_title = path
        .file_stem()
        .and_then(|s| s.to_str())
        .map(|s| s.to_string());

    let tagged_file = match Probe::open(path).and_then(|p| p.read()) {
        Ok(file) => file,
        Err(_) => {
            return FileMetadata {
                title: fallback_title,
                artist: None,
                album: None,
                duration_ms: None,
                bpm: None,
            };
        }
    };

    let properties = tagged_file.properties();
    let duration_ms = Some(properties.duration().as_millis() as u64);

    if let Some(tag) = tagged_file.primary_tag() {
        let title = tag
            .title()
            .map(|s| s.to_string())
            .or(fallback_title);
        let artist = tag.artist().map(|s| s.to_string());
        let album = tag.album().map(|s| s.to_string());
        let bpm = tag
            .get_string(&lofty::tag::ItemKey::Bpm)
            .and_then(|s| s.parse::<f32>().ok());

        return FileMetadata {
            title,
            artist,
            album,
            duration_ms,
            bpm,
        };
    }

    FileMetadata {
        title: fallback_title,
        artist: None,
        album: None,
        duration_ms,
        bpm: None,
    }
}
