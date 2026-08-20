use super::db::LibraryState;
use super::ingest::hash_file;
use serde::Serialize;

/// 健全性チェックの深さ。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HealthDepth {
    /// ファイルの有無だけを見る。ディスクI/Oはメタデータ参照のみ。
    Quick,
    /// 内容ハッシュまで検証する。ライブラリ全体を読み込む。
    Full,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct HealthIssue {
    pub track_id: i64,
    pub kind: String,
    pub path: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct HealthReport {
    pub checked: u32,
    pub missing: u32,
    pub hash_mismatch: u32,
    pub issues: Vec<HealthIssue>,
}

impl HealthReport {
    pub fn has_issues(&self) -> bool {
        self.missing > 0 || self.hash_mismatch > 0
    }
}

pub fn check_health(library: &LibraryState) -> Result<HealthReport, String> {
    check_health_with(library, HealthDepth::Full)
}

pub fn check_health_with(
    library: &LibraryState,
    depth: HealthDepth,
) -> Result<HealthReport, String> {
    let tracks = library
        .list_all_tracks()
        .map_err(|error| error.to_string())?;
    let mut report = HealthReport {
        checked: tracks.len() as u32,
        missing: 0,
        hash_mismatch: 0,
        issues: Vec::new(),
    };

    for track in tracks {
        let absolute = library.absolute_path(&track.stored_path);
        if !absolute.is_file() {
            report.missing += 1;
            report.issues.push(HealthIssue {
                track_id: track.id,
                kind: "missing".to_string(),
                path: track.path,
            });
            continue;
        }
        if depth == HealthDepth::Quick {
            continue;
        }
        let Some(expected) = track.content_hash.as_deref() else {
            continue;
        };
        let actual = hash_file(&absolute)?;
        if actual != expected {
            report.hash_mismatch += 1;
            report.issues.push(HealthIssue {
                track_id: track.id,
                kind: "hashMismatch".to_string(),
                path: track.path,
            });
        }
    }

    Ok(report)
}
