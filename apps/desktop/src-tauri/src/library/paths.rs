use std::path::{Path, PathBuf};

const WINDOWS_ILLEGAL: [char; 9] = ['<', '>', ':', '"', '/', '\\', '|', '?', '*'];

pub fn resolve_library_root() -> PathBuf {
    dirs::audio_dir()
        .or_else(|| dirs::home_dir().map(|home| home.join("Music")))
        .unwrap_or_else(|| PathBuf::from("Music"))
        .join("Catra")
}

pub fn ensure_library_layout(root: &Path) -> Result<(), String> {
    for dir in ["library", "trash", "artwork", "incoming"] {
        std::fs::create_dir_all(root.join(dir)).map_err(|error| error.to_string())?;
    }
    Ok(())
}

pub fn clear_incoming(root: &Path) {
    let incoming = root.join("incoming");
    let Ok(entries) = std::fs::read_dir(&incoming) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            let _ = std::fs::remove_dir_all(&path);
        } else {
            let _ = std::fs::remove_file(&path);
        }
    }
}

pub fn to_relative(root: &Path, path: &Path) -> Option<String> {
    let root_key = normalize_compare_key(root);
    let path_key = normalize_compare_key(path);
    let rest = path_key.strip_prefix(&root_key)?;
    let rest = rest.trim_start_matches(['\\', '/']);
    if rest.is_empty() {
        return None;
    }
    Some(rest.replace('\\', "/"))
}

pub fn to_absolute(root: &Path, relative: &str) -> PathBuf {
    let trimmed = relative.trim_start_matches(['\\', '/']);
    if trimmed.is_empty() {
        return root.to_path_buf();
    }
    let mut absolute = root.to_path_buf();
    for part in trimmed.split(['/', '\\']).filter(|part| !part.is_empty()) {
        absolute.push(part);
    }
    absolute
}

pub fn is_under(root: &Path, path: &Path) -> bool {
    let root_key = normalize_compare_key(root);
    let path_key = normalize_compare_key(path);
    path_key == root_key || path_key.starts_with(&format!("{root_key}\\"))
}

pub fn normalize_compare_key(path: &Path) -> String {
    let mut normalized = path.to_string_lossy().replace('/', "\\");
    while normalized.ends_with('\\') && normalized.len() > 3 {
        normalized.pop();
    }
    #[cfg(windows)]
    {
        normalized.make_ascii_lowercase();
    }
    normalized
}

pub fn sanitize_filename(name: &str) -> String {
    let mut sanitized: String = name
        .chars()
        .map(|ch| {
            if ch.is_control() || WINDOWS_ILLEGAL.contains(&ch) {
                '_'
            } else {
                ch
            }
        })
        .collect();
    while sanitized.ends_with('.') || sanitized.ends_with(' ') {
        sanitized.pop();
    }
    sanitized
}

pub fn filename_for_ingest(source: &Path, track_id: Option<i64>) -> String {
    let original = source
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("");
    let sanitized = sanitize_filename(original);
    let stem = source
        .file_stem()
        .and_then(|stem| stem.to_str())
        .map(sanitize_filename)
        .unwrap_or_default();
    if !sanitized.is_empty() && has_visible_name(&stem) {
        return sanitized;
    }
    let ext = source
        .extension()
        .and_then(|ext| ext.to_str())
        .map(|ext| ext.to_ascii_lowercase())
        .filter(|ext| !ext.is_empty());
    match (track_id, ext) {
        (Some(id), Some(ext)) => format!("{id}.{ext}"),
        (Some(id), None) => id.to_string(),
        (None, Some(ext)) => format!("track.{ext}"),
        (None, None) => "track".to_string(),
    }
}

fn has_visible_name(name: &str) -> bool {
    name.chars().any(|ch| ch.is_alphanumeric())
}

pub fn converted_filename(source: &Path, extension: &str) -> String {
    let stem = source
        .file_stem()
        .and_then(|stem| stem.to_str())
        .map(sanitize_filename)
        .filter(|stem| !stem.is_empty())
        .unwrap_or_else(|| "track".to_string());
    format!("{stem}.{extension}")
}

pub fn canonical_audio_format(path: &Path) -> String {
    match path
        .extension()
        .and_then(|ext| ext.to_str())
        .map(|ext| ext.to_ascii_lowercase())
        .as_deref()
    {
        Some("mp3") | Some("mpga") => "mp3".to_string(),
        Some("flac") => "flac".to_string(),
        Some("wav") => "wav".to_string(),
        Some("aiff") | Some("aif") => "aiff".to_string(),
        Some("m4a") | Some("aac") | Some("mp4") => "m4a".to_string(),
        Some("ogg") => "ogg".to_string(),
        Some("opus") => "opus".to_string(),
        Some("wma") => "wma".to_string(),
        Some(other) => other.to_string(),
        None => String::new(),
    }
}

pub fn is_alt_format(existing: &Path, candidate: &Path) -> bool {
    let existing_format = canonical_audio_format(existing);
    let candidate_format = canonical_audio_format(candidate);
    !existing_format.is_empty()
        && !candidate_format.is_empty()
        && existing_format != candidate_format
}

pub fn library_prefix(id: i64) -> String {
    format!("library/{id}")
}

pub fn trash_prefix(id: i64) -> String {
    format!("trash/{id}")
}

pub fn swap_location_prefix(relative: &str, from_prefix: &str, to_prefix: &str) -> String {
    let from = from_prefix.trim_end_matches('/');
    if let Some(rest) = relative.strip_prefix(from) {
        format!("{to_prefix}{rest}")
    } else {
        relative.to_string()
    }
}

pub fn managed_track_id(relative: &str) -> Option<i64> {
    let mut parts = relative.split('/');
    let kind = parts.next()?;
    if kind != "library" && kind != "trash" {
        return None;
    }
    parts.next()?.parse().ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sanitize_replaces_illegal_windows_chars() {
        assert_eq!(sanitize_filename("a<b>c:d\"e/f\\g|h?i*.mp3"), "a_b_c_d_e_f_g_h_i_.mp3");
    }

    #[test]
    fn sanitize_empty_after_strip_uses_id_fallback() {
        let path = Path::new("???");
        assert_eq!(filename_for_ingest(path, Some(12)), "12");
        let path = Path::new("???.flac");
        assert_eq!(filename_for_ingest(path, Some(12)), "12.flac");
    }

    #[test]
    fn relative_roundtrip_uses_forward_slashes() {
        let root = PathBuf::from(r"C:\Users\dj\Music\Catra");
        let absolute = PathBuf::from(r"C:\Users\dj\Music\Catra\library\4\song.mp3");
        let relative = to_relative(&root, &absolute).unwrap();
        assert_eq!(relative, "library/4/song.mp3");
        assert_eq!(to_absolute(&root, &relative), absolute);
    }

    #[test]
    fn trash_swap_keeps_id_and_filename() {
        let relative = "library/9/track name.wav";
        let trashed = swap_location_prefix(relative, "library", "trash");
        assert_eq!(trashed, "trash/9/track name.wav");
        assert_eq!(
            swap_location_prefix(&trashed, "trash", "library"),
            relative
        );
    }

    #[test]
    fn aiff_and_aif_are_the_same_format() {
        assert!(!is_alt_format(Path::new("a.aiff"), Path::new("b.aif")));
        assert!(is_alt_format(Path::new("a.flac"), Path::new("b.mp3")));
        assert!(!is_alt_format(Path::new("a.mp3"), Path::new("b.mp3")));
    }

    #[test]
    fn converted_filename_keeps_stem() {
        assert_eq!(
            converted_filename(Path::new("song title.flac"), "mp3"),
            "song title.mp3"
        );
    }
}
