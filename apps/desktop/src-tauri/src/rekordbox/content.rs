use super::db::MasterDatabase;
use rusqlite::{params, Row};
use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RekordboxContent {
    pub id: String,
    pub folder_path: String,
    pub file_name: Option<String>,
    pub title: Option<String>,
    pub artist: Option<String>,
    pub album: Option<String>,
    pub genre: Option<String>,
    pub bpm: Option<f32>,
    pub length_secs: Option<i32>,
    pub track_no: Option<i32>,
    pub bit_rate: Option<i32>,
    pub bit_depth: Option<i32>,
    pub comment: Option<String>,
    pub file_type: Option<i32>,
    pub rating: Option<i32>,
    pub release_year: Option<i32>,
    pub key: Option<String>,
    pub remixer: Option<String>,
    pub label: Option<String>,
    pub composer: Option<String>,
    pub file_size: Option<i32>,
    pub disc_no: Option<i32>,
}

const CONTENT_QUERY: &str = "
SELECT
    c.ID,
    c.FolderPath,
    c.FileNameL,
    c.Title,
    artist.Name,
    album.Name,
    genre.Name,
    c.BPM,
    c.Length,
    c.TrackNo,
    c.BitRate,
    c.BitDepth,
    c.Commnt,
    c.FileType,
    c.Rating,
    c.ReleaseYear,
    djmd_key.ScaleName,
    remixer.Name,
    label.Name,
    composer.Name,
    c.FileSize,
    c.DiscNo
FROM djmdContent c
LEFT JOIN djmdArtist artist ON c.ArtistID = artist.ID
LEFT JOIN djmdAlbum album ON c.AlbumID = album.ID
LEFT JOIN djmdGenre genre ON c.GenreID = genre.ID
LEFT JOIN djmdKey djmd_key ON c.KeyID = djmd_key.ID
LEFT JOIN djmdArtist remixer ON c.RemixerID = remixer.ID
LEFT JOIN djmdLabel label ON c.LabelID = label.ID
LEFT JOIN djmdArtist composer ON c.ComposerID = composer.ID
";

impl MasterDatabase {
    pub fn get_content(&self, id: Option<&str>) -> Result<Vec<RekordboxContent>, String> {
        let sql = match id {
            Some(_) => format!("{CONTENT_QUERY} WHERE c.ID = ?1"),
            None => CONTENT_QUERY.to_string(),
        };

        let mut stmt = self
            .conn()
            .prepare(&sql)
            .map_err(|error| error.to_string())?;

        let rows = match id {
            Some(content_id) => stmt.query_map(params![content_id], map_content_row),
            None => stmt.query_map([], map_content_row),
        }
        .map_err(|error| error.to_string())?;

        rows.collect::<Result<Vec<_>, _>>()
            .map_err(|error| error.to_string())
    }
}

fn map_content_row(row: &Row<'_>) -> rusqlite::Result<RekordboxContent> {
    let bpm_raw: Option<i32> = row.get(7)?;
    Ok(RekordboxContent {
        id: row.get(0)?,
        folder_path: row.get(1)?,
        file_name: row.get(2)?,
        title: row.get(3)?,
        artist: row.get(4)?,
        album: row.get(5)?,
        genre: row.get(6)?,
        bpm: bpm_raw.map(|value| value as f32 / 100.0),
        length_secs: row.get(8)?,
        track_no: row.get(9)?,
        bit_rate: row.get(10)?,
        bit_depth: row.get(11)?,
        comment: row.get(12)?,
        file_type: row.get(13)?,
        rating: row.get(14)?,
        release_year: row.get(15)?,
        key: row.get(16)?,
        remixer: row.get(17)?,
        label: row.get(18)?,
        composer: row.get(19)?,
        file_size: row.get(20)?,
        disc_no: row.get(21)?,
    })
}
