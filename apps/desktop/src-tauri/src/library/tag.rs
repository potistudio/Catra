//! ユーザー由来の属性、つまりタグ。
//!
//! タグはファイルに書き戻さない。だから再取り込みで上書きされない（不変条件3）。
//! 階層は「軸 > タグ」の2段で固定で、継承はない。軸が `single` なら、
//! 1曲がその軸で持てるタグは1つだけ。この排他は書き込みのトランザクションの中で守る。

use super::db::LibraryState;
use super::playlist;
use rusqlite::{params, params_from_iter, Connection, OptionalExtension};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum TagSelection {
    Single,
    Multi,
}

impl TagSelection {
    fn as_str(self) -> &'static str {
        match self {
            TagSelection::Single => "single",
            TagSelection::Multi => "multi",
        }
    }

    fn parse(value: &str) -> TagSelection {
        match value {
            "single" => TagSelection::Single,
            _ => TagSelection::Multi,
        }
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Tag {
    pub id: i64,
    pub axis_id: i64,
    pub name: String,
    pub position: i64,
    /// このタグが付いている曲の数。ゴミ箱の中は数えない。
    pub track_count: i64,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TagAxis {
    pub id: i64,
    pub name: String,
    pub selection: TagSelection,
    pub position: i64,
    pub tags: Vec<Tag>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TrackTags {
    pub track_id: i64,
    pub tag_ids: Vec<i64>,
}

fn err(error: impl std::fmt::Display) -> String {
    error.to_string()
}

fn now() -> i64 {
    chrono::Utc::now().timestamp()
}

fn normalize_name(name: &str) -> Result<String, String> {
    let trimmed = name.trim();
    if trimmed.is_empty() {
        return Err("名前が空です".to_string());
    }
    Ok(trimmed.to_string())
}

fn tags_of_axis(conn: &Connection, axis_id: i64) -> Result<Vec<Tag>, String> {
    let mut stmt = conn
        .prepare(
            "SELECT g.id, g.axis_id, g.name, g.position,
                    (SELECT COUNT(*) FROM track_tags tt
                     JOIN tracks t ON t.id = tt.track_id
                     WHERE tt.tag_id = g.id AND t.trashed_at IS NULL)
             FROM tags g WHERE g.axis_id = ?1 ORDER BY g.position, g.id",
        )
        .map_err(err)?;
    let tags = stmt
        .query_map(params![axis_id], |row| {
            Ok(Tag {
                id: row.get(0)?,
                axis_id: row.get(1)?,
                name: row.get(2)?,
                position: row.get(3)?,
                track_count: row.get(4)?,
            })
        })
        .map_err(err)?
        .collect::<rusqlite::Result<Vec<_>>>()
        .map_err(err)?;
    Ok(tags)
}

fn axis_by_id(conn: &Connection, id: i64) -> Result<TagAxis, String> {
    let row: Option<(i64, String, String, i64)> = conn
        .query_row(
            "SELECT id, name, selection, position FROM tag_axes WHERE id = ?1",
            params![id],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)),
        )
        .optional()
        .map_err(err)?;
    let Some((id, name, selection, position)) = row else {
        return Err(format!("軸が見つかりません: {id}"));
    };
    Ok(TagAxis {
        id,
        name,
        selection: TagSelection::parse(&selection),
        position,
        tags: tags_of_axis(conn, id)?,
    })
}

fn tag_by_id(conn: &Connection, id: i64) -> Result<Tag, String> {
    conn.query_row(
        "SELECT g.id, g.axis_id, g.name, g.position,
                (SELECT COUNT(*) FROM track_tags tt
                 JOIN tracks t ON t.id = tt.track_id
                 WHERE tt.tag_id = g.id AND t.trashed_at IS NULL)
         FROM tags g WHERE g.id = ?1",
        params![id],
        |row| {
            Ok(Tag {
                id: row.get(0)?,
                axis_id: row.get(1)?,
                name: row.get(2)?,
                position: row.get(3)?,
                track_count: row.get(4)?,
            })
        },
    )
    .optional()
    .map_err(err)?
    .ok_or_else(|| format!("タグが見つかりません: {id}"))
}

pub fn list_axes(library: &LibraryState) -> Result<Vec<TagAxis>, String> {
    let conn = library.conn().map_err(err)?;
    let ids: Vec<i64> = {
        let mut stmt = conn
            .prepare("SELECT id FROM tag_axes ORDER BY position, id")
            .map_err(err)?;
        let ids = stmt
            .query_map([], |row| row.get(0))
            .map_err(err)?
            .collect::<rusqlite::Result<Vec<_>>>()
            .map_err(err)?;
        ids
    };
    ids.into_iter().map(|id| axis_by_id(&conn, id)).collect()
}

pub fn create_axis(
    library: &LibraryState,
    name: &str,
    selection: TagSelection,
) -> Result<TagAxis, String> {
    let name = normalize_name(name)?;
    let conn = library.conn().map_err(err)?;
    let position: i64 = conn
        .query_row(
            "SELECT COALESCE(MAX(position) + 1, 0) FROM tag_axes",
            [],
            |row| row.get(0),
        )
        .map_err(err)?;
    let stamp = now();
    conn.execute(
        "INSERT INTO tag_axes (name, selection, position, created_at, updated_at)
         VALUES (?1, ?2, ?3, ?4, ?4)",
        params![name, selection.as_str(), position, stamp],
    )
    .map_err(|error| match error {
        rusqlite::Error::SqliteFailure(_, _) => format!("同じ名前の軸があります: {name}"),
        other => other.to_string(),
    })?;
    axis_by_id(&conn, conn.last_insert_rowid())
}

/// multi から single へ切り替えるとき、既に2つ以上のタグを持っている曲の数を数える。
/// 0でなければ切り替えは通らない。どれを残すかはユーザーが決めることである。
pub fn axis_conflicts(library: &LibraryState, axis_id: i64) -> Result<u32, String> {
    let conn = library.conn().map_err(err)?;
    conn.query_row(
        "SELECT COUNT(*) FROM (
             SELECT tt.track_id FROM track_tags tt
             JOIN tags g ON g.id = tt.tag_id
             WHERE g.axis_id = ?1
             GROUP BY tt.track_id HAVING COUNT(*) > 1
         )",
        params![axis_id],
        |row| row.get(0),
    )
    .map_err(err)
}

pub fn update_axis(
    library: &LibraryState,
    id: i64,
    name: Option<&str>,
    selection: Option<TagSelection>,
) -> Result<TagAxis, String> {
    if selection == Some(TagSelection::Single) {
        let conflicts = axis_conflicts(library, id)?;
        if conflicts > 0 {
            return Err(format!(
                "この軸で2つ以上のタグを持つ曲が {conflicts} 曲あります。先に整理してください"
            ));
        }
    }

    let conn = library.conn().map_err(err)?;
    if let Some(name) = name {
        let name = normalize_name(name)?;
        conn.execute(
            "UPDATE tag_axes SET name = ?2, updated_at = ?3 WHERE id = ?1",
            params![id, name, now()],
        )
        .map_err(err)?;
    }
    if let Some(selection) = selection {
        conn.execute(
            "UPDATE tag_axes SET selection = ?2, updated_at = ?3 WHERE id = ?1",
            params![id, selection.as_str(), now()],
        )
        .map_err(err)?;
    }
    axis_by_id(&conn, id)
}

/// 軸を消すと、その下のタグと割り当ても消える。曲は消えない。
/// 消えたタグを指していた述語の項は落とす。
pub fn delete_axis(library: &LibraryState, id: i64) -> Result<u32, String> {
    let mut guard = library.conn().map_err(err)?;
    let tx = guard.transaction().map_err(err)?;
    let dead_tags: Vec<i64> = {
        let mut stmt = tx
            .prepare("SELECT id FROM tags WHERE axis_id = ?1")
            .map_err(err)?;
        let ids = stmt
            .query_map(params![id], |row| row.get(0))
            .map_err(err)?
            .collect::<rusqlite::Result<Vec<_>>>()
            .map_err(err)?;
        ids
    };
    let removed = tx
        .execute("DELETE FROM tag_axes WHERE id = ?1", params![id])
        .map_err(err)?;
    if removed == 0 {
        return Err(format!("軸が見つかりません: {id}"));
    }
    playlist::prune_dead_refs(&tx, &dead_tags, &[])?;
    tx.commit().map_err(err)?;
    Ok(dead_tags.len() as u32)
}

pub fn create_tag(library: &LibraryState, axis_id: i64, name: &str) -> Result<Tag, String> {
    let name = normalize_name(name)?;
    let conn = library.conn().map_err(err)?;
    let axis_exists: bool = conn
        .query_row(
            "SELECT 1 FROM tag_axes WHERE id = ?1",
            params![axis_id],
            |_| Ok(true),
        )
        .optional()
        .map_err(err)?
        .unwrap_or(false);
    if !axis_exists {
        return Err(format!("軸が見つかりません: {axis_id}"));
    }

    let position: i64 = conn
        .query_row(
            "SELECT COALESCE(MAX(position) + 1, 0) FROM tags WHERE axis_id = ?1",
            params![axis_id],
            |row| row.get(0),
        )
        .map_err(err)?;
    let stamp = now();
    conn.execute(
        "INSERT INTO tags (axis_id, name, position, created_at, updated_at)
         VALUES (?1, ?2, ?3, ?4, ?4)",
        params![axis_id, name, position, stamp],
    )
    .map_err(|_| format!("同じ軸に同じ名前のタグがあります: {name}"))?;
    tag_by_id(&conn, conn.last_insert_rowid())
}

pub fn rename_tag(library: &LibraryState, id: i64, name: &str) -> Result<Tag, String> {
    let name = normalize_name(name)?;
    let conn = library.conn().map_err(err)?;
    let changed = conn
        .execute(
            "UPDATE tags SET name = ?2, updated_at = ?3 WHERE id = ?1",
            params![id, name, now()],
        )
        .map_err(|_| format!("同じ軸に同じ名前のタグがあります: {name}"))?;
    if changed == 0 {
        return Err(format!("タグが見つかりません: {id}"));
    }
    tag_by_id(&conn, id)
}

pub fn delete_tag(library: &LibraryState, id: i64) -> Result<(), String> {
    let mut guard = library.conn().map_err(err)?;
    let tx = guard.transaction().map_err(err)?;
    let removed = tx
        .execute("DELETE FROM tags WHERE id = ?1", params![id])
        .map_err(err)?;
    if removed == 0 {
        return Err(format!("タグが見つかりません: {id}"));
    }
    playlist::prune_dead_refs(&tx, &[id], &[])?;
    tx.commit().map_err(err)?;
    Ok(())
}

/// タグを付ける。single の軸では、同じ軸の他のタグを同じトランザクションの中で外す。
/// 「付けたら前のが外れた」が1回の操作として見えるようにする。
pub fn assign(library: &LibraryState, track_ids: &[i64], tag_ids: &[i64]) -> Result<(), String> {
    if track_ids.is_empty() || tag_ids.is_empty() {
        return Ok(());
    }
    let mut guard = library.conn().map_err(err)?;
    let tx = guard.transaction().map_err(err)?;
    let stamp = now();

    for tag_id in tag_ids {
        let axis: Option<(i64, String)> = tx
            .query_row(
                "SELECT a.id, a.selection FROM tags g JOIN tag_axes a ON a.id = g.axis_id
                 WHERE g.id = ?1",
                params![tag_id],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .optional()
            .map_err(err)?;
        let Some((axis_id, selection)) = axis else {
            return Err(format!("タグが見つかりません: {tag_id}"));
        };

        if TagSelection::parse(&selection) == TagSelection::Single {
            let placeholders = vec!["?"; track_ids.len()].join(", ");
            let mut values: Vec<i64> = vec![axis_id];
            values.extend_from_slice(track_ids);
            tx.execute(
                &format!(
                    "DELETE FROM track_tags
                     WHERE tag_id IN (SELECT id FROM tags WHERE axis_id = ?)
                       AND track_id IN ({placeholders})"
                ),
                params_from_iter(values.iter()),
            )
            .map_err(err)?;
        }

        for track_id in track_ids {
            tx.execute(
                "INSERT OR IGNORE INTO track_tags (track_id, tag_id, assigned_at)
                 VALUES (?1, ?2, ?3)",
                params![track_id, tag_id, stamp],
            )
            .map_err(err)?;
        }
    }

    tx.commit().map_err(err)?;
    Ok(())
}

pub fn unassign(library: &LibraryState, track_ids: &[i64], tag_ids: &[i64]) -> Result<u32, String> {
    if track_ids.is_empty() || tag_ids.is_empty() {
        return Ok(0);
    }
    let conn = library.conn().map_err(err)?;
    let track_slots = vec!["?"; track_ids.len()].join(", ");
    let tag_slots = vec!["?"; tag_ids.len()].join(", ");
    let mut values: Vec<i64> = track_ids.to_vec();
    values.extend_from_slice(tag_ids);
    let removed = conn
        .execute(
            &format!(
                "DELETE FROM track_tags
                 WHERE track_id IN ({track_slots}) AND tag_id IN ({tag_slots})"
            ),
            params_from_iter(values.iter()),
        )
        .map_err(err)?;
    Ok(removed as u32)
}

/// 選択中の曲がどのタグを持っているかを返す。タグパネルの点灯に使う。
pub fn of_tracks(library: &LibraryState, track_ids: &[i64]) -> Result<Vec<TrackTags>, String> {
    if track_ids.is_empty() {
        return Ok(Vec::new());
    }
    let conn = library.conn().map_err(err)?;
    let placeholders = vec!["?"; track_ids.len()].join(", ");
    let mut stmt = conn
        .prepare(&format!(
            "SELECT track_id, tag_id FROM track_tags
             WHERE track_id IN ({placeholders}) ORDER BY track_id, tag_id"
        ))
        .map_err(err)?;
    let pairs = stmt
        .query_map(params_from_iter(track_ids.iter()), |row| {
            Ok((row.get::<_, i64>(0)?, row.get::<_, i64>(1)?))
        })
        .map_err(err)?
        .collect::<rusqlite::Result<Vec<_>>>()
        .map_err(err)?;

    let mut out: Vec<TrackTags> = track_ids
        .iter()
        .map(|track_id| TrackTags {
            track_id: *track_id,
            tag_ids: Vec::new(),
        })
        .collect();
    for (track_id, tag_id) in pairs {
        if let Some(slot) = out.iter_mut().find(|item| item.track_id == track_id) {
            slot.tag_ids.push(tag_id);
        }
    }
    Ok(out)
}

/// タグで絞り込んだ結果。複数のタグは AND で効く（全部持っている曲だけ残る）。
/// その場の絞り込みなので、どこにも保存しない。
pub fn track_ids(library: &LibraryState, tag_ids: &[i64]) -> Result<Vec<i64>, String> {
    if tag_ids.is_empty() {
        return Ok(Vec::new());
    }
    let conn = library.conn().map_err(err)?;
    let placeholders = vec!["?"; tag_ids.len()].join(", ");
    let mut stmt = conn
        .prepare(&format!(
            "SELECT t.id FROM tracks t
             JOIN track_tags tt ON tt.track_id = t.id
             WHERE t.trashed_at IS NULL AND tt.tag_id IN ({placeholders})
             GROUP BY t.id
             HAVING COUNT(DISTINCT tt.tag_id) = {count}
             ORDER BY t.artist COLLATE NOCASE, t.title COLLATE NOCASE",
            count = tag_ids.len()
        ))
        .map_err(err)?;
    let rows = stmt
        .query_map(params_from_iter(tag_ids.iter()), |row| row.get(0))
        .map_err(err)?
        .collect::<rusqlite::Result<Vec<i64>>>()
        .map_err(err)?;
    Ok(rows)
}

/// 重複解決で負けた側のタグを勝ち側へ引き継ぐ。
/// single の軸で勝ち側が既にタグを持っていたら、勝ち側のものを残す。
pub(super) fn carry_over_tags(
    conn: &Connection,
    from_track_id: i64,
    to_track_id: i64,
) -> Result<(), String> {
    conn.execute(
        "INSERT OR IGNORE INTO track_tags (track_id, tag_id, assigned_at)
         SELECT ?2, tt.tag_id, tt.assigned_at
         FROM track_tags tt
         JOIN tags g ON g.id = tt.tag_id
         JOIN tag_axes a ON a.id = g.axis_id
         WHERE tt.track_id = ?1
           AND (a.selection != 'single'
                OR NOT EXISTS (
                    SELECT 1 FROM track_tags won
                    JOIN tags won_tag ON won_tag.id = won.tag_id
                    WHERE won.track_id = ?2 AND won_tag.axis_id = g.axis_id
                ))",
        params![from_track_id, to_track_id],
    )
    .map_err(err)?;
    Ok(())
}

/// 述語がまだ生きているタグを指しているかを確かめる。規則の保存前に呼ぶ。
pub(super) fn missing_tags(conn: &Connection, ids: &[i64]) -> Result<Vec<i64>, String> {
    let mut missing = Vec::new();
    for id in ids {
        let exists: bool = conn
            .query_row("SELECT 1 FROM tags WHERE id = ?1", params![id], |_| {
                Ok(true)
            })
            .optional()
            .map_err(err)?
            .unwrap_or(false);
        if !exists {
            missing.push(*id);
        }
    }
    Ok(missing)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::library::db::NewTrack;
    use uuid::Uuid;

    fn temp_library() -> LibraryState {
        let root = std::env::temp_dir().join(format!("catra-tag-{}", Uuid::new_v4()));
        std::fs::create_dir_all(&root).unwrap();
        LibraryState::new(root).unwrap()
    }

    fn insert_track(library: &LibraryState, title: &str) -> i64 {
        library
            .insert_track(NewTrack {
                path: &format!("library/{}.wav", Uuid::new_v4()),
                title: Some(title),
                artist: Some("Artist"),
                album: None,
                duration_ms: Some(1000),
                bpm: None,
                bitrate_kbps: None,
                genre: None,
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
    fn filtering_by_several_tags_keeps_only_tracks_that_hold_all_of_them() {
        let library = temp_library();
        let axis = create_axis(&library, "テーマ", TagSelection::Multi).unwrap();
        let warm = create_tag(&library, axis.id, "暖").unwrap();
        let night = create_tag(&library, axis.id, "夜").unwrap();
        let both = insert_track(&library, "Both");
        let one = insert_track(&library, "One");

        assign(&library, &[both], &[warm.id, night.id]).unwrap();
        assign(&library, &[one], &[warm.id]).unwrap();

        assert_eq!(track_ids(&library, &[warm.id]).unwrap().len(), 2);
        assert_eq!(
            track_ids(&library, &[warm.id, night.id]).unwrap(),
            vec![both]
        );
    }

    #[test]
    fn a_single_axis_holds_only_one_tag_per_track() {
        let library = temp_library();
        let axis = create_axis(&library, "テンション", TagSelection::Single).unwrap();
        let low = create_tag(&library, axis.id, "静").unwrap();
        let high = create_tag(&library, axis.id, "動").unwrap();
        let song = insert_track(&library, "Song");

        assign(&library, &[song], &[low.id]).unwrap();
        assign(&library, &[song], &[high.id]).unwrap();

        let held = of_tracks(&library, &[song]).unwrap();
        assert_eq!(held[0].tag_ids, vec![high.id]);
    }

    #[test]
    fn a_multi_axis_stacks_tags() {
        let library = temp_library();
        let axis = create_axis(&library, "テーマ", TagSelection::Multi).unwrap();
        let a = create_tag(&library, axis.id, "夏").unwrap();
        let b = create_tag(&library, axis.id, "夜").unwrap();
        let song = insert_track(&library, "Song");

        assign(&library, &[song], &[a.id, b.id]).unwrap();
        let held = of_tracks(&library, &[song]).unwrap();
        assert_eq!(held[0].tag_ids.len(), 2);
    }

    #[test]
    fn switching_to_single_is_blocked_while_a_track_holds_two() {
        let library = temp_library();
        let axis = create_axis(&library, "テーマ", TagSelection::Multi).unwrap();
        let a = create_tag(&library, axis.id, "夏").unwrap();
        let b = create_tag(&library, axis.id, "夜").unwrap();
        let song = insert_track(&library, "Song");
        assign(&library, &[song], &[a.id, b.id]).unwrap();

        assert_eq!(axis_conflicts(&library, axis.id).unwrap(), 1);
        assert!(update_axis(&library, axis.id, None, Some(TagSelection::Single)).is_err());

        unassign(&library, &[song], &[b.id]).unwrap();
        assert!(update_axis(&library, axis.id, None, Some(TagSelection::Single)).is_ok());
    }

    #[test]
    fn deleting_an_axis_takes_its_tags_but_not_the_songs() {
        let library = temp_library();
        let axis = create_axis(&library, "テーマ", TagSelection::Multi).unwrap();
        let tag = create_tag(&library, axis.id, "夏").unwrap();
        let song = insert_track(&library, "Song");
        assign(&library, &[song], &[tag.id]).unwrap();

        assert_eq!(delete_axis(&library, axis.id).unwrap(), 1);
        assert!(list_axes(&library).unwrap().is_empty());
        assert!(library.get_track(song).unwrap().is_some());
        assert!(of_tracks(&library, &[song]).unwrap()[0].tag_ids.is_empty());
    }

    #[test]
    fn the_winner_keeps_its_own_tag_on_a_single_axis() {
        let library = temp_library();
        let axis = create_axis(&library, "テンション", TagSelection::Single).unwrap();
        let low = create_tag(&library, axis.id, "静").unwrap();
        let high = create_tag(&library, axis.id, "動").unwrap();
        let old = insert_track(&library, "Old");
        let new = insert_track(&library, "New");
        assign(&library, &[old], &[low.id]).unwrap();
        assign(&library, &[new], &[high.id]).unwrap();

        let conn = library.conn().unwrap();
        carry_over_tags(&conn, old, new).unwrap();
        drop(conn);

        let held = of_tracks(&library, &[new]).unwrap();
        assert_eq!(held[0].tag_ids, vec![high.id]);
    }

    #[test]
    fn carry_over_hands_tags_to_a_bare_winner() {
        let library = temp_library();
        let axis = create_axis(&library, "テーマ", TagSelection::Multi).unwrap();
        let a = create_tag(&library, axis.id, "夏").unwrap();
        let b = create_tag(&library, axis.id, "夜").unwrap();
        let old = insert_track(&library, "Old");
        let new = insert_track(&library, "New");
        assign(&library, &[old], &[a.id, b.id]).unwrap();

        let conn = library.conn().unwrap();
        carry_over_tags(&conn, old, new).unwrap();
        drop(conn);

        assert_eq!(of_tracks(&library, &[new]).unwrap()[0].tag_ids.len(), 2);
    }

    #[test]
    fn tag_counts_ignore_the_trash() {
        let library = temp_library();
        let axis = create_axis(&library, "テーマ", TagSelection::Multi).unwrap();
        let tag = create_tag(&library, axis.id, "夏").unwrap();
        let song = insert_track(&library, "Song");
        assign(&library, &[song], &[tag.id]).unwrap();
        assert_eq!(list_axes(&library).unwrap()[0].tags[0].track_count, 1);

        library
            .conn()
            .unwrap()
            .execute(
                "UPDATE tracks SET trashed_at = 1 WHERE id = ?1",
                params![song],
            )
            .unwrap();
        assert_eq!(list_axes(&library).unwrap()[0].tags[0].track_count, 0);
    }
}
