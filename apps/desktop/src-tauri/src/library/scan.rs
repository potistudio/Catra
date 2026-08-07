use super::db::{save_artwork, LibraryState, ScanResult};
use lofty::file::AudioFile;
use lofty::file::TaggedFileExt;
use lofty::picture::PictureType;
use lofty::probe::Probe;
use lofty::tag::Accessor;
use lofty::tag::ItemKey;
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

        let artwork_path = metadata
            .artwork
            .as_ref()
            .and_then(|(data, mime)| save_artwork(library.artwork_dir(), &path_str, data, mime));

        let inserted = library
            .insert_track(
                &path_str,
                metadata.title.as_deref(),
                metadata.artist.as_deref(),
                metadata.album.as_deref(),
                metadata.duration_ms,
                metadata.bpm,
                metadata.bitrate_kbps,
                metadata.genre.as_deref(),
                metadata.key.as_deref(),
                metadata.rating,
                artwork_path.as_deref(),
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
    bitrate_kbps: Option<u32>,
    genre: Option<String>,
    key: Option<String>,
    rating: Option<u8>,
    artwork: Option<(Vec<u8>, String)>,
}

fn read_metadata(path: &Path) -> FileMetadata {
    let fallback_title = path
        .file_stem()
        .and_then(|s| s.to_str())
        .map(|s| s.to_string());

    let tagged_file = match Probe::open(path).and_then(|p| p.read()) {
        Ok(file) => file,
        Err(_) => {
            return empty_metadata(fallback_title);
        }
    };

    let properties = tagged_file.properties();
    let duration_ms = Some(properties.duration().as_millis() as u64);
    let bitrate_kbps = properties.audio_bitrate();

    if let Some(tag) = tagged_file.primary_tag() {
        let title = tag.title().map(|s| s.to_string()).or(fallback_title);
        let artist = tag.artist().map(|s| s.to_string());
        let album = tag.album().map(|s| s.to_string());
        let genre = tag.genre().map(|s| s.to_string());
        let key = tag
            .get_string(&ItemKey::InitialKey)
            .map(|s| s.to_string());
        let bpm = tag
            .get_string(&ItemKey::Bpm)
            .or_else(|| tag.get_string(&ItemKey::IntegerBpm))
            .and_then(|s| s.parse::<f32>().ok());
        let rating = parse_rating(tag);
        let artwork = extract_artwork(tag);

        return FileMetadata {
            title,
            artist,
            album,
            duration_ms,
            bpm,
            bitrate_kbps,
            genre,
            key,
            rating,
            artwork,
        };
    }

    FileMetadata {
        title: fallback_title,
        artist: None,
        album: None,
        duration_ms,
        bpm: None,
        bitrate_kbps,
        genre: None,
        key: None,
        rating: None,
        artwork: None,
    }
}

fn empty_metadata(fallback_title: Option<String>) -> FileMetadata {
    FileMetadata {
        title: fallback_title,
        artist: None,
        album: None,
        duration_ms: None,
        bpm: None,
        bitrate_kbps: None,
        genre: None,
        key: None,
        rating: None,
        artwork: None,
    }
}

fn parse_rating(tag: &lofty::tag::Tag) -> Option<u8> {
    if let Some(item) = tag.get(&ItemKey::Popularimeter) {
        if let Some(binary) = item.value().binary() {
            if let Some(null_pos) = binary.iter().position(|&b| b == 0) {
                let rating_index = null_pos + 1;
                if rating_index < binary.len() {
                    let rating = binary[rating_index];
                    if rating > 0 {
                        return Some(rating);
                    }
                }
            }
        }

        if let Some(text) = item.value().text() {
            if let Ok(rating) = text.parse::<u8>() {
                if rating > 0 {
                    return Some(rating);
                }
            }
        }
    }

    None
}

fn extract_artwork(tag: &lofty::tag::Tag) -> Option<(Vec<u8>, String)> {
    let picture = tag
        .get_picture_type(PictureType::CoverFront)
        .or_else(|| tag.pictures().first())?;

    let mime = picture
        .mime_type()
        .map(|m| m.as_str().to_string())
        .unwrap_or_else(|| "image/jpeg".to_string());

    Some((picture.data().to_vec(), mime))
}
