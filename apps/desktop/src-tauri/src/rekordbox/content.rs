use super::db::MasterDatabase;
use rusqlite::{params, Row};
use serde::Serialize;
use std::path::{Path, PathBuf};

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
    pub artwork_path: Option<String>,
}

pub(crate) const CONTENT_QUERY: &str = "
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
    c.DiscNo,
    c.ImagePath
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
    pub fn get_content(
        &self,
        id: Option<&str>,
        db_dir: &Path,
    ) -> Result<Vec<RekordboxContent>, String> {
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

pub(crate) fn map_content_row(row: &Row<'_>) -> rusqlite::Result<RekordboxContent> {
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
        rating: normalize_rating(row.get(14)?),
        release_year: row.get(15)?,
        key: row.get(16)?,
        remixer: row.get(17)?,
        label: row.get(18)?,
        composer: row.get(19)?,
        file_size: row.get(20)?,
        disc_no: row.get(21)?,
        // Temporary: relative ImagePath; resolved in get_content.
        artwork_path: row.get(22)?,
    })
}

/// Map `djmdContent.Rating` onto the 0-255 scale used by the UI and ID3 POPM.
///
/// `master.db` stores star counts 0-5. Rekordbox XML uses 0/51/102/153/204/255.
fn normalize_rating(rating: Option<i32>) -> Option<i32> {
    match rating {
        None => None,
        Some(stars) if (0..=5).contains(&stars) => Some(stars * 51),
        Some(value) => Some(value.clamp(0, 255)),
    }
}

/// Resolve Rekordbox `ImagePath` to an absolute file path under the DB directory.
pub(crate) fn resolve_artwork_path(db_dir: &Path, image_path: Option<&str>) -> Option<String> {
    let relative = normalize_image_path(image_path?)?;
    let path = Path::new(relative);

    let candidates: Vec<PathBuf> = if path.is_absolute() {
        vec![path.to_path_buf()]
    } else {
        // ImagePath is stored like `/PIONEER/Artwork/.../artwork.jpg` (share-relative).
        vec![
            db_dir.join("share").join(path),
            db_dir.join(path),
        ]
    };

    for candidate in candidates {
        if let Some(resolved) = prefer_medium_artwork(&candidate) {
            return Some(resolved.to_string_lossy().into_owned());
        }
    }

    None
}

fn normalize_image_path(image_path: &str) -> Option<&str> {
    let trimmed = image_path.trim().trim_start_matches(['/', '\\']);
    if trimmed.is_empty() {
        None
    } else {
        Some(trimmed)
    }
}

fn prefer_medium_artwork(path: &Path) -> Option<PathBuf> {
    if let (Some(stem), Some(ext)) = (
        path.file_stem().and_then(|value| value.to_str()),
        path.extension().and_then(|value| value.to_str()),
    ) {
        if !stem.ends_with("_m") && !stem.ends_with("_s") {
            let medium = path.with_file_name(format!("{stem}_m.{ext}"));
            if medium.is_file() {
                return Some(medium);
            }
        }
    }

    path.is_file().then(|| path.to_path_buf())
}

#[cfg(test)]
mod tests {
    use super::{normalize_rating, prefer_medium_artwork, resolve_artwork_path};
    use std::fs;
    use std::path::PathBuf;

    #[test]
    fn normalizes_star_count_rating_to_255_scale() {
        assert_eq!(normalize_rating(None), None);
        assert_eq!(normalize_rating(Some(0)), Some(0));
        assert_eq!(normalize_rating(Some(1)), Some(51));
        assert_eq!(normalize_rating(Some(2)), Some(102));
        assert_eq!(normalize_rating(Some(3)), Some(153));
        assert_eq!(normalize_rating(Some(4)), Some(204));
        assert_eq!(normalize_rating(Some(5)), Some(255));
    }

    #[test]
    fn preserves_xml_style_255_scale_rating() {
        assert_eq!(normalize_rating(Some(51)), Some(51));
        assert_eq!(normalize_rating(Some(255)), Some(255));
        assert_eq!(normalize_rating(Some(300)), Some(255));
        assert_eq!(normalize_rating(Some(-1)), Some(0));
    }

    #[test]
    fn resolves_leading_slash_share_relative_path() {
        let root = std::env::temp_dir().join(format!(
            "catra-rb-art-slash-{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let art_dir = root.join("share/PIONEER/Artwork/000/abc");
        fs::create_dir_all(&art_dir).unwrap();
        let jpg = art_dir.join("artwork.jpg");
        let medium = art_dir.join("artwork_m.jpg");
        fs::write(&jpg, b"full").unwrap();
        fs::write(&medium, b"med").unwrap();

        let resolved = resolve_artwork_path(
            &root,
            Some("/PIONEER/Artwork/000/abc/artwork.jpg"),
        )
        .expect("resolved");
        assert_eq!(PathBuf::from(resolved), medium);

        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn resolves_relative_image_path_under_share() {
        let root = std::env::temp_dir().join(format!(
            "catra-rb-art-{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let art_dir = root.join("share/PIONEER/Artwork/000/abc");
        fs::create_dir_all(&art_dir).unwrap();
        let jpg = art_dir.join("artwork.jpg");
        let medium = art_dir.join("artwork_m.jpg");
        fs::write(&jpg, b"full").unwrap();
        fs::write(&medium, b"med").unwrap();

        let resolved = resolve_artwork_path(
            &root,
            Some("share/PIONEER/Artwork/000/abc/artwork.jpg"),
        )
        .expect("resolved");
        assert_eq!(PathBuf::from(resolved), medium);

        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn prefers_medium_when_present() {
        let root = std::env::temp_dir().join(format!(
            "catra-rb-art-med-{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        fs::create_dir_all(&root).unwrap();
        let jpg = root.join("artwork.jpg");
        let medium = root.join("artwork_m.jpg");
        fs::write(&jpg, b"full").unwrap();
        fs::write(&medium, b"med").unwrap();
        assert_eq!(prefer_medium_artwork(&jpg), Some(medium));
        let _ = fs::remove_dir_all(root);
    }
}

