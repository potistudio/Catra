//! 同値類のブラウズ。アルバムやジャンルは、フィールドの値が同じ曲を勝手に束ねたものである。
//! 実体として登録しないので、集めるのは常にその場の集計になる。

use super::db::LibraryState;
use rusqlite::params;
use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FacetValue {
    /// 値。空欄の曲は `null` にまとめる（「未設定」という束）。
    pub value: Option<String>,
    pub count: i64,
}

/// 束ねられるフィールド。同値類として意味があるものだけを並べる。
/// タイトルは1曲1束になるだけなので出さない。
fn column_for(field: &str) -> Option<&'static str> {
    match field {
        "album" => Some("album"),
        "artist" => Some("artist"),
        "genre" => Some("genre"),
        "key" => Some("key_name"),
        "source" => Some("source"),
        _ => None,
    }
}

pub fn browse_facet(library: &LibraryState, field: &str) -> Result<Vec<FacetValue>, String> {
    let column = column_for(field).ok_or_else(|| format!("束ねられないフィールドです: {field}"))?;
    let conn = library.conn().map_err(|error| error.to_string())?;

    let mut stmt = conn
        .prepare(&format!(
            "SELECT NULLIF(TRIM(COALESCE({column}, '')), '') AS value, COUNT(*)
             FROM tracks WHERE trashed_at IS NULL
             GROUP BY value COLLATE NOCASE
             ORDER BY value IS NULL, value COLLATE NOCASE"
        ))
        .map_err(|error| error.to_string())?;
    let values = stmt
        .query_map([], |row| {
            Ok(FacetValue {
                value: row.get(0)?,
                count: row.get(1)?,
            })
        })
        .map_err(|error| error.to_string())?
        .collect::<rusqlite::Result<Vec<_>>>()
        .map_err(|error| error.to_string())?;
    Ok(values)
}

/// 同値類そのものの中身。値を選んだときに曲を引くのに使う。
pub fn facet_track_ids(
    library: &LibraryState,
    field: &str,
    value: Option<&str>,
) -> Result<Vec<i64>, String> {
    let column = column_for(field).ok_or_else(|| format!("束ねられないフィールドです: {field}"))?;
    let conn = library.conn().map_err(|error| error.to_string())?;

    // 「未設定」の束は空文字で引く。空欄も空白だけも同じ束に落ちる。
    let needle = value.unwrap_or("").trim();
    let mut stmt = conn
        .prepare(&format!(
            "SELECT id FROM tracks
             WHERE trashed_at IS NULL AND TRIM(COALESCE({column}, '')) = ?1 COLLATE NOCASE
             ORDER BY artist COLLATE NOCASE, title COLLATE NOCASE"
        ))
        .map_err(|error| error.to_string())?;
    let rows = stmt
        .query_map(params![needle], |row| row.get(0))
        .map_err(|error| error.to_string())?
        .collect::<rusqlite::Result<Vec<i64>>>()
        .map_err(|error| error.to_string())?;
    Ok(rows)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::library::db::NewTrack;
    use uuid::Uuid;

    fn temp_library() -> LibraryState {
        let root = std::env::temp_dir().join(format!("catra-facet-{}", Uuid::new_v4()));
        std::fs::create_dir_all(&root).unwrap();
        LibraryState::new(root).unwrap()
    }

    fn insert_track(library: &LibraryState, genre: Option<&str>) -> i64 {
        library
            .insert_track(NewTrack {
                path: &format!("library/{}.wav", Uuid::new_v4()),
                title: Some("Song"),
                artist: Some("Artist"),
                album: None,
                duration_ms: Some(1000),
                bpm: None,
                bitrate_kbps: None,
                genre,
                key: None,
                rating: None,
                artwork_path: None,
                source: None,
                converted: false,
                added_at: 1,
                content_hash: None,
                parent_track_id: None,
                format_group_id: None,
                rekordbox_content_id: None,
            })
            .unwrap()
    }

    #[test]
    fn equal_values_fall_into_one_bucket() {
        let library = temp_library();
        insert_track(&library, Some("House"));
        insert_track(&library, Some("house"));
        insert_track(&library, Some("Techno"));

        let facets = browse_facet(&library, "genre").unwrap();
        assert_eq!(facets.len(), 2);
        assert_eq!(facets[0].count, 2);
    }

    #[test]
    fn blank_values_become_one_unset_bucket_at_the_end() {
        let library = temp_library();
        insert_track(&library, Some("House"));
        insert_track(&library, None);
        insert_track(&library, Some("  "));

        let facets = browse_facet(&library, "genre").unwrap();
        let last = facets.last().unwrap();
        assert!(last.value.is_none());
        assert_eq!(last.count, 2);
    }

    #[test]
    fn a_bucket_can_be_opened() {
        let library = temp_library();
        let first = insert_track(&library, Some("House"));
        insert_track(&library, Some("Techno"));
        let ids = facet_track_ids(&library, "genre", Some("house")).unwrap();
        assert_eq!(ids, vec![first]);
    }

    #[test]
    fn a_field_that_is_not_an_equivalence_class_is_rejected() {
        let library = temp_library();
        assert!(browse_facet(&library, "title").is_err());
    }
}
