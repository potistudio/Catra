use super::content::{map_content_row, resolve_artwork_path, CONTENT_QUERY, RekordboxContent};
use super::db::MasterDatabase;
use rusqlite::{params, Row};
use serde::Serialize;
use std::path::Path;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RekordboxPlaylist {
    pub id: String,
    pub name: String,
    pub attribute: i32,
    pub parent_id: Option<String>,
    pub seq: i32,
}

impl MasterDatabase {
    pub fn list_playlists(&self) -> Result<Vec<RekordboxPlaylist>, String> {
        let mut stmt = self
            .conn()
            .prepare(
                "SELECT ID, Name, Attribute, ParentID, Seq
                 FROM djmdPlaylist
                 ORDER BY ParentID, Seq",
            )
            .map_err(|error| error.to_string())?;

        let rows = stmt
            .query_map([], map_playlist_row)
            .map_err(|error| error.to_string())?;

        rows.collect::<Result<Vec<_>, _>>()
            .map_err(|error| error.to_string())
    }

    pub fn get_playlist_content(
        &self,
        playlist_id: &str,
        db_dir: &Path,
    ) -> Result<Vec<RekordboxContent>, String> {
        let sql = format!(
            "{CONTENT_QUERY}
INNER JOIN djmdSongPlaylist sp ON sp.ContentID = c.ID
WHERE sp.PlaylistID = ?1
ORDER BY sp.TrackNo"
        );

        let mut stmt = self
            .conn()
            .prepare(&sql)
            .map_err(|error| error.to_string())?;

        let rows = stmt
            .query_map(params![playlist_id], map_content_row)
            .map_err(|error| error.to_string())?;

        let mut contents = rows
            .collect::<Result<Vec<_>, _>>()
            .map_err(|error| error.to_string())?;

        for content in &mut contents {
            content.artwork_path =
                resolve_artwork_path(db_dir, content.artwork_path.as_deref());
        }

        Ok(contents)
    }
}

fn map_playlist_row(row: &Row<'_>) -> rusqlite::Result<RekordboxPlaylist> {
    let parent_id: Option<String> = row.get(3)?;
    Ok(RekordboxPlaylist {
        id: row.get(0)?,
        name: row.get(1)?,
        attribute: row.get(2)?,
        parent_id: normalize_parent_id(parent_id),
        seq: row.get::<_, Option<i32>>(4)?.unwrap_or(0),
    })
}

fn normalize_parent_id(parent_id: Option<String>) -> Option<String> {
    parent_id.and_then(|value| {
        let trimmed = value.trim();
        if trimmed.is_empty() || trimmed.eq_ignore_ascii_case("root") || trimmed == "0" {
            None
        } else {
            Some(trimmed.to_string())
        }
    })
}
