use super::db::LibraryState;
use lofty::config::ParseOptions;
use lofty::file::TaggedFileExt;
use lofty::probe::Probe;
use lofty::tag::{ItemKey, Tag};
use rusqlite::{params, params_from_iter, OptionalExtension};
use std::collections::BTreeSet;
use std::path::Path;

pub const MAX_SAVED_TRACK_URLS: usize = 200;

pub fn normalize_soundcloud_track_url(value: &str) -> Option<String> {
    let url = tauri::Url::parse(value.trim()).ok()?;
    if !matches!(url.scheme(), "http" | "https")
        || !matches!(
            url.host_str()?,
            "soundcloud.com" | "www.soundcloud.com" | "m.soundcloud.com"
        )
        || !url.username().is_empty()
        || url.password().is_some()
        || url.port().is_some()
    {
        return None;
    }
    let mut parts: Vec<&str> = url
        .path_segments()?
        .filter(|part| !part.is_empty())
        .collect();
    if parts.first() == Some(&"n") {
        parts.remove(0);
    }
    if parts.len() == 3 && parts[2].starts_with("s-") {
        parts.pop();
    }
    if parts.len() != 2
        || parts.iter().any(|part| {
            matches!(
                *part,
                "discover"
                    | "search"
                    | "stream"
                    | "upload"
                    | "feed"
                    | "you"
                    | "sets"
                    | "likes"
                    | "albums"
                    | "tracks"
                    | "reposts"
                    | "comments"
                    | "popular-tracks"
                    | "following"
                    | "followers"
                    | "library"
                    | "notifications"
                    | "messages"
                    | "settings"
                    | "signin"
                    | "signup"
                    | "pages"
                    | "stations"
                    | "charts"
            )
        })
    {
        return None;
    }
    Some(format!("https://soundcloud.com/{}/{}", parts[0], parts[1]))
}

fn source_url_from_tag(tag: &Tag) -> Option<String> {
    // 説明文や購入先の宣伝リンクを、保存した楽曲の同一性として扱わない。
    let keys = [
        ItemKey::AudioSourceUrl,
        ItemKey::PodcastUrl,
        ItemKey::Unknown("purl".into()),
        ItemKey::Unknown("PURL".into()),
        ItemKey::Unknown("URL".into()),
        ItemKey::Unknown("url".into()),
    ];
    for key in keys {
        for item in tag.get_items(&key) {
            if let Some(value) = item.value().text().or_else(|| item.value().locator()) {
                if tauri::Url::parse(value.trim())
                    .is_ok_and(|url| matches!(url.scheme(), "http" | "https"))
                {
                    return normalize_soundcloud_track_url(value);
                }
            }
        }
    }
    let fallback_keys = [
        ItemKey::AudioFileUrl,
        ItemKey::Comment,
        ItemKey::Unknown("comment".into()),
        ItemKey::Unknown("COMMENT".into()),
    ];
    for key in fallback_keys {
        for item in tag.get_items(&key) {
            let value = item.value();
            if let Some(url) = value
                .text()
                .or_else(|| value.locator())
                .and_then(normalize_soundcloud_track_url)
            {
                return Some(url);
            }
        }
    }
    None
}

fn read_source_url(path: &Path) -> Option<String> {
    let options = ParseOptions::new()
        .read_cover_art(false)
        .max_junk_bytes(4096);
    let file = Probe::open(path)
        .and_then(|probe| probe.options(options).read())
        .ok()?;
    file.primary_tag().and_then(source_url_from_tag)
}

pub fn index_pending_sources(library: &LibraryState) -> Result<usize, String> {
    let pending = {
        let conn = library.conn().map_err(|error| error.to_string())?;
        let mut stmt = conn
            .prepare(
                "SELECT t.id, t.path, t.parent_track_id FROM tracks t
             LEFT JOIN track_origins o ON o.track_id = t.id
             WHERE t.trashed_at IS NULL AND o.track_id IS NULL ORDER BY t.id",
            )
            .map_err(|error| error.to_string())?;
        let rows = stmt
            .query_map([], |row| {
                Ok((
                    row.get::<_, i64>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, Option<i64>>(2)?,
                ))
            })
            .map_err(|error| error.to_string())?
            .collect::<rusqlite::Result<Vec<_>>>()
            .map_err(|error| error.to_string())?;
        rows
    };
    let mut indexed = 0;
    for (id, relative, parent_id) in pending {
        let path = library.absolute_path(&relative);
        if !path.is_file() {
            continue;
        }
        // ファイル解析中はライブラリ接続のロックを保持しない。
        let mut source_url = read_source_url(&path);
        let conn = library.conn().map_err(|error| error.to_string())?;
        if source_url.is_none() {
            if let Some(parent_id) = parent_id {
                source_url = conn
                    .query_row(
                        "SELECT source_url FROM track_origins WHERE track_id = ?1",
                        params![parent_id],
                        |row| row.get::<_, Option<String>>(0),
                    )
                    .optional()
                    .map_err(|error| error.to_string())?
                    .flatten();
            }
        }
        indexed += conn
            .execute(
                "INSERT INTO track_origins (track_id, source_url)
             SELECT id, ?2 FROM tracks WHERE id = ?1
             ON CONFLICT(track_id) DO NOTHING",
                params![id, source_url],
            )
            .map_err(|error| error.to_string())?;
    }
    Ok(indexed)
}

pub fn saved_soundcloud_urls(
    library: &LibraryState,
    urls: &[String],
) -> Result<Vec<String>, String> {
    if urls.len() > MAX_SAVED_TRACK_URLS || urls.iter().any(|url| url.len() > 2048) {
        return Err("照合するURLが多すぎるか、長すぎます".into());
    }
    let requested: BTreeSet<String> = urls
        .iter()
        .filter_map(|url| normalize_soundcloud_track_url(url))
        .collect();
    if requested.is_empty() {
        return Ok(Vec::new());
    }
    let matches = {
        let conn = library.conn().map_err(|error| error.to_string())?;
        let slots = vec!["?"; requested.len()].join(",");
        let mut stmt = conn
            .prepare(&format!(
                "SELECT o.source_url, t.path FROM track_origins o
             JOIN tracks t ON t.id = o.track_id
             WHERE t.trashed_at IS NULL AND o.source_url IN ({slots})"
            ))
            .map_err(|error| error.to_string())?;
        let rows = stmt
            .query_map(params_from_iter(requested.iter()), |row| {
                Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
            })
            .map_err(|error| error.to_string())?
            .collect::<rusqlite::Result<Vec<_>>>()
            .map_err(|error| error.to_string())?;
        rows
    };
    Ok(matches
        .into_iter()
        .filter_map(|(url, path)| library.absolute_path(&path).is_file().then_some(url))
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::library::db::NewTrack;
    use lofty::tag::{ItemValue, TagItem, TagType};
    use std::fs;

    fn temp_library() -> LibraryState {
        LibraryState::new(
            std::env::temp_dir().join(format!("catra-source-{}", uuid::Uuid::new_v4())),
        )
        .unwrap()
    }

    fn insert_file(library: &LibraryState, parent: Option<i64>) -> i64 {
        let path = format!("library/{}.mp3", uuid::Uuid::new_v4());
        fs::write(library.absolute_path(&path), b"audio").unwrap();
        library
            .insert_track(NewTrack {
                path: &path,
                title: Some("Song"),
                artist: Some("Artist"),
                album: None,
                duration_ms: None,
                bpm: None,
                bitrate_kbps: None,
                genre: None,
                key: None,
                rating: None,
                artwork_path: None,
                source: Some("soundcloud"),
                converted: parent.is_some(),
                added_at: 1,
                content_hash: None,
                parent_track_id: parent,
                format_group_id: None,
                rekordbox_content_id: None,
            })
            .unwrap()
    }

    fn set_origin(library: &LibraryState, id: i64, url: &str) {
        library
            .conn()
            .unwrap()
            .execute(
                "INSERT INTO track_origins (track_id, source_url) VALUES (?1, ?2)",
                params![id, url],
            )
            .unwrap();
    }

    #[test]
    fn membership_requires_an_active_track_with_an_existing_audio_file() {
        let library = temp_library();
        let id = insert_file(&library, None);
        let url = "https://soundcloud.com/artist/song";
        set_origin(&library, id, url);
        let requested = vec![format!("{url}?si=tracking"), url.to_string()];
        assert_eq!(
            saved_soundcloud_urls(&library, &requested).unwrap(),
            vec![url]
        );
        assert!(saved_soundcloud_urls(&library, &[]).unwrap().is_empty());
        assert!(
            saved_soundcloud_urls(&library, &["https://soundcloud.com/other/song".into()])
                .unwrap()
                .is_empty()
        );
        library.set_trashed(id, Some(1)).unwrap();
        assert!(saved_soundcloud_urls(&library, &requested)
            .unwrap()
            .is_empty());
        library.set_trashed(id, None).unwrap();
        let track = library.get_track(id).unwrap().unwrap();
        fs::remove_file(&track.path).unwrap();
        assert!(saved_soundcloud_urls(&library, &requested)
            .unwrap()
            .is_empty());
        fs::write(&track.path, b"audio").unwrap();
        assert_eq!(
            saved_soundcloud_urls(&library, &requested).unwrap(),
            vec![url]
        );
        library.delete_track_row(id).unwrap();
        assert!(saved_soundcloud_urls(&library, &requested)
            .unwrap()
            .is_empty());
        assert_eq!(
            library
                .conn()
                .unwrap()
                .query_row("SELECT COUNT(*) FROM track_origins", [], |row| row
                    .get::<_, i64>(0))
                .unwrap(),
            0
        );
    }

    #[test]
    fn another_active_format_keeps_the_saved_mark() {
        let library = temp_library();
        let first = insert_file(&library, None);
        let second = insert_file(&library, None);
        let url = "https://soundcloud.com/artist/song";
        set_origin(&library, first, url);
        set_origin(&library, second, url);
        library.set_trashed(first, Some(1)).unwrap();
        assert_eq!(
            saved_soundcloud_urls(&library, &[url.into()]).unwrap(),
            vec![url]
        );
    }

    #[test]
    fn indexing_is_persistent_and_converted_tracks_inherit_the_origin() {
        let library = temp_library();
        let parent = insert_file(&library, None);
        let child = insert_file(&library, Some(parent));
        let unknown = insert_file(&library, None);
        let url = "https://soundcloud.com/artist/song";
        set_origin(&library, parent, url);
        assert_eq!(index_pending_sources(&library).unwrap(), 2);
        assert_eq!(index_pending_sources(&library).unwrap(), 0);
        let conn = library.conn().unwrap();
        let child_origin: String = conn
            .query_row(
                "SELECT source_url FROM track_origins WHERE track_id = ?1",
                params![child],
                |row| row.get(0),
            )
            .unwrap();
        let unknown_origin: Option<String> = conn
            .query_row(
                "SELECT source_url FROM track_origins WHERE track_id = ?1",
                params![unknown],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(child_origin, url);
        assert_eq!(unknown_origin, None);
        drop(conn);
        library.set_trashed(parent, Some(1)).unwrap();
        assert_eq!(
            saved_soundcloud_urls(&library, &[url.into()]).unwrap(),
            vec![url]
        );
    }

    #[test]
    fn request_limits_prevent_unbounded_library_queries() {
        let library = temp_library();
        assert!(saved_soundcloud_urls(
            &library,
            &vec!["https://soundcloud.com/artist/song".into(); 201]
        )
        .is_err());
        assert!(saved_soundcloud_urls(&library, &["x".repeat(2049)]).is_err());
        assert!(
            saved_soundcloud_urls(&library, &["https://evil.test/artist/song".into()])
                .unwrap()
                .is_empty()
        );
    }

    #[test]
    fn canonical_urls_ignore_tracking_frames_and_private_tokens() {
        for input in [
            "https://soundcloud.com/artist/song?si=tracking#player",
            "http://www.soundcloud.com/artist/song/",
            "https://m.soundcloud.com/artist/song",
            "https://soundcloud.com/n/artist/song?embedded=true",
            "https://soundcloud.com/artist/song/s-secret",
        ] {
            assert_eq!(
                normalize_soundcloud_track_url(input).as_deref(),
                Some("https://soundcloud.com/artist/song")
            );
        }
    }

    #[test]
    fn non_tracks_and_untrusted_hosts_are_not_matched() {
        for input in [
            "https://soundcloud.com/artist",
            "https://soundcloud.com/artist/sets/album",
            "https://soundcloud.com/artist/likes",
            "https://soundcloud.com/discover/sets",
            "https://soundcloud.com/artist/song/likes",
            "https://soundcloud.com.evil.test/artist/song",
            "https://evil.test/artist/song",
            "https://user@soundcloud.com/artist/song",
            "https://soundcloud.com:8000/artist/song",
            "file:///artist/song",
            "not a URL",
        ] {
            assert_eq!(normalize_soundcloud_track_url(input), None, "{input}");
        }
    }

    #[test]
    fn non_soundcloud_origins_do_not_fall_back_to_comment_promotions() {
        let mut tag = Tag::new(TagType::Id3v2);
        tag.insert_unchecked(TagItem::new(
            ItemKey::Unknown("purl".into()),
            ItemValue::Text("https://artist.bandcamp.com/track/song".into()),
        ));
        tag.insert_unchecked(TagItem::new(
            ItemKey::Comment,
            ItemValue::Text("https://soundcloud.com/other/promotion".into()),
        ));
        assert_eq!(source_url_from_tag(&tag), None);
    }

    #[test]
    fn only_explicit_track_origins_are_indexed() {
        let mut tag = Tag::new(TagType::Id3v2);
        tag.insert_unchecked(TagItem::new(
            ItemKey::Description,
            ItemValue::Text("https://soundcloud.com/other/promotion".into()),
        ));
        tag.insert_unchecked(TagItem::new(
            ItemKey::TrackArtistUrl,
            ItemValue::Locator("https://soundcloud.com/other/track".into()),
        ));
        assert_eq!(source_url_from_tag(&tag), None);
        tag.insert_unchecked(TagItem::new(
            ItemKey::Unknown("purl".into()),
            ItemValue::Text("https://soundcloud.com/artist/song?si=x".into()),
        ));
        assert_eq!(
            source_url_from_tag(&tag).as_deref(),
            Some("https://soundcloud.com/artist/song")
        );
    }
}
