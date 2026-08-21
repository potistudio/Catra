//! Catra のプレイリスト。木の操作と、列への要素の挿入・削除・並べ替え。
//!
//! 静的プレイリストだけが**列**で、要素は `playlist_entries.id` という自分の同一性を持つ。
//! スマートプレイリストとフォルダは**集合**で、要素の同一性はトラックそのものである。
//! この非対称が、このモジュールのほとんどの分岐の理由になっている。

use super::db::{map_track_row, LibraryState, Track, TRACK_COLUMNS};
use super::rule;
use super::tag;
use rusqlite::{params, params_from_iter, Connection, OptionalExtension};
use serde::{Deserialize, Serialize};
use serde_json::Value;

const NODE_COLUMNS: &str =
    "id, parent_id, kind, name, position, rule, sort_rule, created_at, updated_at";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum PlaylistKind {
    Folder,
    Static,
    Smart,
}

impl PlaylistKind {
    fn as_str(self) -> &'static str {
        match self {
            PlaylistKind::Folder => "folder",
            PlaylistKind::Static => "static",
            PlaylistKind::Smart => "smart",
        }
    }

    fn parse(value: &str) -> Result<Self, String> {
        match value {
            "folder" => Ok(PlaylistKind::Folder),
            "static" => Ok(PlaylistKind::Static),
            "smart" => Ok(PlaylistKind::Smart),
            other => Err(format!("プレイリストの種類が不正です: {other}")),
        }
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PlaylistNode {
    pub id: i64,
    pub parent_id: Option<i64>,
    pub kind: PlaylistKind,
    pub name: String,
    pub position: i64,
    /// static は列の長さ（重複を重複のまま数える）。smart と folder は集合の大きさ。
    pub track_count: i64,
    pub rule: Option<Value>,
    pub sort_rule: Option<Value>,
    pub created_at: i64,
    pub updated_at: i64,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PlaylistEntry {
    /// 要素の同一性。static のみ。smart / folder は null で、並べ替えできないことを表す。
    pub entry_id: Option<i64>,
    pub position: i64,
    pub track: Track,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PlaylistRef {
    pub id: i64,
    pub name: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DeleteImpact {
    /// 自分を含めて消えるノードの数。
    pub nodes: u32,
    /// 消える要素の総数。曲は消えない。
    pub entries: u32,
    /// この削除で述語が壊れるスマートプレイリスト。
    pub referencing: Vec<PlaylistRef>,
}

struct NodeRow {
    id: i64,
    parent_id: Option<i64>,
    kind: PlaylistKind,
    name: String,
    position: i64,
    rule: Option<Value>,
    sort_rule: Option<Value>,
    created_at: i64,
    updated_at: i64,
}

fn err(error: impl std::fmt::Display) -> String {
    error.to_string()
}

fn now() -> i64 {
    chrono::Utc::now().timestamp()
}

fn map_node_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<NodeRow> {
    let kind: String = row.get(2)?;
    let rule: Option<String> = row.get(5)?;
    let sort_rule: Option<String> = row.get(6)?;
    Ok(NodeRow {
        id: row.get(0)?,
        parent_id: row.get(1)?,
        kind: PlaylistKind::parse(&kind).unwrap_or(PlaylistKind::Folder),
        name: row.get(3)?,
        position: row.get(4)?,
        rule: rule.and_then(|raw| serde_json::from_str(&raw).ok()),
        sort_rule: sort_rule.and_then(|raw| serde_json::from_str(&raw).ok()),
        created_at: row.get(7)?,
        updated_at: row.get(8)?,
    })
}

/// 曲の列を引くときの SELECT 句。`map_track_row` は位置で読むので順番を崩せない。
fn track_columns(alias: &str) -> String {
    TRACK_COLUMNS
        .split(',')
        .map(|column| format!("{alias}.{}", column.trim()))
        .collect::<Vec<_>>()
        .join(", ")
}

fn load_row(conn: &Connection, id: i64) -> Result<NodeRow, String> {
    conn.query_row(
        &format!("SELECT {NODE_COLUMNS} FROM playlists WHERE id = ?1"),
        params![id],
        map_node_row,
    )
    .optional()
    .map_err(err)?
    .ok_or_else(|| format!("プレイリストが見つかりません: {id}"))
}

/// 種類ごとの数え方。static は列なので重複を数え、smart と folder は集合なので1曲1回。
fn count_of(conn: &Connection, row: &NodeRow) -> Result<i64, String> {
    match row.kind {
        PlaylistKind::Static => conn
            .query_row(
                "SELECT COUNT(*) FROM playlist_entries e
                 JOIN tracks t ON t.id = e.track_id
                 WHERE e.playlist_id = ?1 AND t.trashed_at IS NULL",
                params![row.id],
                |r| r.get(0),
            )
            .map_err(err),
        PlaylistKind::Smart => {
            let Some(rule) = row.rule.as_ref() else {
                return Ok(0);
            };
            let query = rule::build_rule_query(conn, rule, Some(row.id))?;
            conn.query_row(
                &format!(
                    "SELECT COUNT(*) FROM tracks t
                     WHERE t.trashed_at IS NULL AND ({})",
                    query.where_sql
                ),
                params_from_iter(query.params.iter()),
                |r| r.get(0),
            )
            .map_err(err)
        }
        PlaylistKind::Folder => {
            let membership = serde_json::json!({ "in_playlist": row.id });
            let query = rule::build_rule_query(conn, &membership, None)?;
            conn.query_row(
                &format!(
                    "SELECT COUNT(*) FROM tracks t
                     WHERE t.trashed_at IS NULL AND ({})",
                    query.where_sql
                ),
                params_from_iter(query.params.iter()),
                |r| r.get(0),
            )
            .map_err(err)
        }
    }
}

fn to_node(conn: &Connection, row: NodeRow) -> Result<PlaylistNode, String> {
    let track_count = count_of(conn, &row)?;
    Ok(PlaylistNode {
        id: row.id,
        parent_id: row.parent_id,
        kind: row.kind,
        name: row.name,
        position: row.position,
        track_count,
        rule: row.rule,
        sort_rule: row.sort_rule,
        created_at: row.created_at,
        updated_at: row.updated_at,
    })
}

fn node_by_id(conn: &Connection, id: i64) -> Result<PlaylistNode, String> {
    let row = load_row(conn, id)?;
    to_node(conn, row)
}

fn normalize_name(name: &str) -> Result<String, String> {
    let trimmed = name.trim();
    if trimmed.is_empty() {
        return Err("名前が空です".to_string());
    }
    Ok(trimmed.to_string())
}

fn ensure_folder(conn: &Connection, parent_id: Option<i64>) -> Result<(), String> {
    let Some(parent_id) = parent_id else {
        return Ok(());
    };
    let parent = load_row(conn, parent_id)?;
    if parent.kind != PlaylistKind::Folder {
        return Err("フォルダ以外の中にはプレイリストを置けません".to_string());
    }
    Ok(())
}

/// 木は非巡回でなければならない。移動先が自分自身か自分の子孫なら弾く。
fn ensure_not_descendant(conn: &Connection, id: i64, parent_id: Option<i64>) -> Result<(), String> {
    let mut cursor = parent_id;
    while let Some(current) = cursor {
        if current == id {
            return Err("自分自身の中には移動できません".to_string());
        }
        cursor = conn
            .query_row(
                "SELECT parent_id FROM playlists WHERE id = ?1",
                params![current],
                |row| row.get::<_, Option<i64>>(0),
            )
            .optional()
            .map_err(err)?
            .flatten();
    }
    Ok(())
}

/// 並びを先に読んでから番号を振る。1つの UPDATE で数え直すと、
/// 途中まで書き換えた表を数えることになって番号がぶつかる。
fn renumber(conn: &Connection, select: &str, table: &str, key: i64) -> Result<(), String> {
    let ordered: Vec<i64> = {
        let mut stmt = conn.prepare(select).map_err(err)?;
        let ids = stmt
            .query_map(params![key], |row| row.get(0))
            .map_err(err)?
            .collect::<rusqlite::Result<Vec<_>>>()
            .map_err(err)?;
        ids
    };
    for (index, id) in ordered.iter().enumerate() {
        conn.execute(
            &format!("UPDATE {table} SET position = ?2 WHERE id = ?1"),
            params![id, index as i64],
        )
        .map_err(err)?;
    }
    Ok(())
}

/// 兄弟の `position` を0から連番に振り直す。並びは (position, id) で安定させる。
fn renumber_siblings(conn: &Connection, parent_id: Option<i64>) -> Result<(), String> {
    let ordered: Vec<i64> = {
        let mut stmt = conn
            .prepare("SELECT id FROM playlists WHERE parent_id IS ?1 ORDER BY position, id")
            .map_err(err)?;
        let ids = stmt
            .query_map(params![parent_id], |row| row.get(0))
            .map_err(err)?
            .collect::<rusqlite::Result<Vec<_>>>()
            .map_err(err)?;
        ids
    };
    for (index, id) in ordered.iter().enumerate() {
        conn.execute(
            "UPDATE playlists SET position = ?2 WHERE id = ?1",
            params![id, index as i64],
        )
        .map_err(err)?;
    }
    Ok(())
}

/// 要素の `position` を0から連番に振り直す。挿入・削除・並べ替えの直後に必ず呼ぶ。
fn renumber_entries(conn: &Connection, playlist_id: i64) -> Result<(), String> {
    renumber(
        conn,
        "SELECT id FROM playlist_entries WHERE playlist_id = ?1 ORDER BY position, id",
        "playlist_entries",
        playlist_id,
    )
}

fn subtree_ids(conn: &Connection, id: i64) -> Result<Vec<i64>, String> {
    let mut stmt = conn
        .prepare(
            "WITH RECURSIVE sub(id) AS (
                 SELECT id FROM playlists WHERE id = ?1
                 UNION ALL
                 SELECT p.id FROM playlists p JOIN sub ON p.parent_id = sub.id
             ) SELECT id FROM sub",
        )
        .map_err(err)?;
    let ids = stmt
        .query_map(params![id], |row| row.get(0))
        .map_err(err)?
        .collect::<rusqlite::Result<Vec<i64>>>()
        .map_err(err)?;
    Ok(ids)
}

/// 消えた参照を指す項を、他のスマートプレイリストの述語から落とす。
pub(super) fn prune_dead_refs(
    conn: &Connection,
    dead_tags: &[i64],
    dead_playlists: &[i64],
) -> Result<(), String> {
    let rules: Vec<(i64, String)> = {
        let mut stmt = conn
            .prepare("SELECT id, rule FROM playlists WHERE kind = 'smart' AND rule IS NOT NULL")
            .map_err(err)?;
        let rows = stmt
            .query_map([], |row| Ok((row.get(0)?, row.get(1)?)))
            .map_err(err)?
            .collect::<rusqlite::Result<Vec<_>>>()
            .map_err(err)?;
        rows
    };

    for (id, raw) in rules {
        if dead_playlists.contains(&id) {
            continue;
        }
        let Ok(parsed) = serde_json::from_str::<Value>(&raw) else {
            continue;
        };
        let mut tags = Vec::new();
        let mut playlists = Vec::new();
        rule::collect_refs(&parsed, &mut tags, &mut playlists);
        let touched = tags.iter().any(|tag| dead_tags.contains(tag))
            || playlists.iter().any(|list| dead_playlists.contains(list));
        if !touched {
            continue;
        }
        let pruned = rule::prune_refs(&parsed, dead_tags, dead_playlists);
        let encoded = pruned.map(|value| value.to_string());
        conn.execute(
            "UPDATE playlists SET rule = ?2, updated_at = ?3 WHERE id = ?1",
            params![id, encoded, now()],
        )
        .map_err(err)?;
    }
    Ok(())
}

// ---------------------------------------------------------------- 木の操作

pub fn list_tree(library: &LibraryState) -> Result<Vec<PlaylistNode>, String> {
    let conn = library.conn().map_err(err)?;
    let rows: Vec<NodeRow> = {
        let mut stmt = conn
            .prepare(&format!(
                "SELECT {NODE_COLUMNS} FROM playlists
                 ORDER BY parent_id IS NOT NULL, parent_id, position, id"
            ))
            .map_err(err)?;
        let rows = stmt
            .query_map([], map_node_row)
            .map_err(err)?
            .collect::<rusqlite::Result<Vec<_>>>()
            .map_err(err)?;
        rows
    };

    let mut nodes = Vec::with_capacity(rows.len());
    for row in rows {
        nodes.push(to_node(&conn, row)?);
    }
    Ok(nodes)
}

pub fn create(
    library: &LibraryState,
    name: &str,
    parent_id: Option<i64>,
    kind: PlaylistKind,
) -> Result<PlaylistNode, String> {
    let name = normalize_name(name)?;
    let mut guard = library.conn().map_err(err)?;
    let tx = guard.transaction().map_err(err)?;

    ensure_folder(&tx, parent_id)?;
    let position: i64 = tx
        .query_row(
            "SELECT COALESCE(MAX(position) + 1, 0) FROM playlists WHERE parent_id IS ?1",
            params![parent_id],
            |row| row.get(0),
        )
        .map_err(err)?;

    let stamp = now();
    tx.execute(
        "INSERT INTO playlists (parent_id, kind, name, position, rule, sort_rule, created_at, updated_at)
         VALUES (?1, ?2, ?3, ?4, NULL, NULL, ?5, ?5)",
        params![parent_id, kind.as_str(), name, position, stamp],
    )
    .map_err(err)?;
    let id = tx.last_insert_rowid();
    let node = node_by_id(&tx, id)?;
    tx.commit().map_err(err)?;
    Ok(node)
}

pub fn rename(library: &LibraryState, id: i64, name: &str) -> Result<PlaylistNode, String> {
    let name = normalize_name(name)?;
    let conn = library.conn().map_err(err)?;
    let changed = conn
        .execute(
            "UPDATE playlists SET name = ?2, updated_at = ?3 WHERE id = ?1",
            params![id, name, now()],
        )
        .map_err(err)?;
    if changed == 0 {
        return Err(format!("プレイリストが見つかりません: {id}"));
    }
    node_by_id(&conn, id)
}

pub fn move_node(
    library: &LibraryState,
    id: i64,
    parent_id: Option<i64>,
    position: i64,
) -> Result<(), String> {
    let mut guard = library.conn().map_err(err)?;
    let tx = guard.transaction().map_err(err)?;

    let current = load_row(&tx, id)?;
    ensure_folder(&tx, parent_id)?;
    ensure_not_descendant(&tx, id, parent_id)?;

    // 既存の兄弟を偶数の席に広げ、移す子を奇数の席に差し込む。
    // 振り直しで席は0からの連番に戻るので、この倍々は一時的な足場でしかない。
    tx.execute(
        "UPDATE playlists SET position = position * 2 WHERE parent_id IS ?1",
        params![parent_id],
    )
    .map_err(err)?;
    tx.execute(
        "UPDATE playlists SET parent_id = ?2, position = ?3, updated_at = ?4 WHERE id = ?1",
        params![id, parent_id, position.max(0) * 2 - 1, now()],
    )
    .map_err(err)?;

    renumber_siblings(&tx, parent_id)?;
    if current.parent_id != parent_id {
        renumber_siblings(&tx, current.parent_id)?;
    }
    tx.commit().map_err(err)?;
    Ok(())
}

pub fn delete_impact(library: &LibraryState, id: i64) -> Result<DeleteImpact, String> {
    let conn = library.conn().map_err(err)?;
    let ids = subtree_ids(&conn, id)?;
    if ids.is_empty() {
        return Err(format!("プレイリストが見つかりません: {id}"));
    }

    let placeholders = vec!["?"; ids.len()].join(", ");
    let entries: u32 = conn
        .query_row(
            &format!("SELECT COUNT(*) FROM playlist_entries WHERE playlist_id IN ({placeholders})"),
            params_from_iter(ids.iter()),
            |row| row.get(0),
        )
        .map_err(err)?;

    let mut referencing = Vec::new();
    {
        let mut stmt = conn
            .prepare(
                "SELECT id, name, rule FROM playlists WHERE kind = 'smart' AND rule IS NOT NULL",
            )
            .map_err(err)?;
        let rows = stmt
            .query_map([], |row| {
                Ok((
                    row.get::<_, i64>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                ))
            })
            .map_err(err)?
            .collect::<rusqlite::Result<Vec<_>>>()
            .map_err(err)?;
        for (other_id, name, raw) in rows {
            if ids.contains(&other_id) {
                continue;
            }
            let Ok(parsed) = serde_json::from_str::<Value>(&raw) else {
                continue;
            };
            let mut tags = Vec::new();
            let mut playlists = Vec::new();
            rule::collect_refs(&parsed, &mut tags, &mut playlists);
            if playlists.iter().any(|list| ids.contains(list)) {
                referencing.push(PlaylistRef { id: other_id, name });
            }
        }
    }

    Ok(DeleteImpact {
        nodes: ids.len() as u32,
        entries,
        referencing,
    })
}

/// 木ごと消す。曲は消えない。消えるのは所属だけである。
pub fn delete(library: &LibraryState, id: i64) -> Result<u32, String> {
    let mut guard = library.conn().map_err(err)?;
    let tx = guard.transaction().map_err(err)?;

    let row = load_row(&tx, id)?;
    let ids = subtree_ids(&tx, id)?;
    // 子は ON DELETE CASCADE で落ちる。要素も同じ経路で落ちる。
    tx.execute("DELETE FROM playlists WHERE id = ?1", params![id])
        .map_err(err)?;
    prune_dead_refs(&tx, &[], &ids)?;
    renumber_siblings(&tx, row.parent_id)?;
    tx.commit().map_err(err)?;
    Ok(ids.len() as u32)
}

pub fn set_rule(
    library: &LibraryState,
    id: i64,
    rule_value: Option<Value>,
    sort_rule: Option<Value>,
) -> Result<PlaylistNode, String> {
    let conn = library.conn().map_err(err)?;
    let row = load_row(&conn, id)?;
    if row.kind != PlaylistKind::Smart {
        return Err("規則を持てるのはスマートプレイリストだけです".to_string());
    }

    // 保存前に組み立てて、循環と未知のフィールドをここで弾く。
    if let Some(value) = rule_value.as_ref() {
        rule::build_rule_query(&conn, value, Some(id))?;
        let mut tags = Vec::new();
        let mut playlists = Vec::new();
        rule::collect_refs(value, &mut tags, &mut playlists);
        let missing = tag::missing_tags(&conn, &tags)?;
        if !missing.is_empty() {
            return Err(format!("消えたタグを指しています: {missing:?}"));
        }
    }
    if let Some(value) = sort_rule.as_ref() {
        let column = value.get("column").and_then(Value::as_str);
        match column.and_then(rule::column_name) {
            Some(_) => {}
            None => return Err("並べ替えに使えない列です".to_string()),
        }
    }

    conn.execute(
        "UPDATE playlists SET rule = ?2, sort_rule = ?3, updated_at = ?4 WHERE id = ?1",
        params![
            id,
            rule_value.as_ref().map(Value::to_string),
            sort_rule.as_ref().map(Value::to_string),
            now()
        ],
    )
    .map_err(err)?;
    node_by_id(&conn, id)
}

// ------------------------------------------------------------ 要素の読み出し

fn order_clause(sort_rule: Option<&Value>) -> String {
    let default = "t.artist COLLATE NOCASE, t.title COLLATE NOCASE".to_string();
    let Some(object) = sort_rule.and_then(Value::as_object) else {
        return default;
    };
    let Some(column) = object
        .get("column")
        .and_then(Value::as_str)
        .and_then(rule::column_name)
    else {
        return default;
    };
    let direction = match object.get("direction").and_then(Value::as_str) {
        Some("desc") => "DESC",
        _ => "ASC",
    };
    format!("t.{column} {direction}, t.title COLLATE NOCASE")
}

/// 種類を隠して要素を返す。呼ぶ側は static かどうかを知らなくてよい。
/// static だけが `entryId` を持つ。持たないものは並べ替えできない、という意味になる。
pub fn entries(library: &LibraryState, id: i64) -> Result<Vec<PlaylistEntry>, String> {
    let conn = library.conn().map_err(err)?;
    let row = load_row(&conn, id)?;
    let columns = track_columns("t");

    match row.kind {
        PlaylistKind::Static => {
            let mut stmt = conn
                .prepare(&format!(
                    "SELECT {columns}, e.id, e.position FROM playlist_entries e
                     JOIN tracks t ON t.id = e.track_id
                     WHERE e.playlist_id = ?1 AND t.trashed_at IS NULL
                     ORDER BY e.position, e.id"
                ))
                .map_err(err)?;
            let rows = stmt
                .query_map(params![id], |row| {
                    Ok((
                        map_track_row(row)?,
                        row.get::<_, i64>(20)?,
                        row.get::<_, i64>(21)?,
                    ))
                })
                .map_err(err)?
                .collect::<rusqlite::Result<Vec<_>>>()
                .map_err(err)?;
            Ok(rows
                .into_iter()
                .map(|(track, entry_id, position)| PlaylistEntry {
                    entry_id: Some(entry_id),
                    position,
                    track: library.expand_track(track),
                })
                .collect())
        }
        PlaylistKind::Smart | PlaylistKind::Folder => {
            let (query, order) = if row.kind == PlaylistKind::Smart {
                let Some(rule_value) = row.rule.as_ref() else {
                    return Ok(Vec::new());
                };
                (
                    rule::build_rule_query(&conn, rule_value, Some(id))?,
                    order_clause(row.sort_rule.as_ref()),
                )
            } else {
                let membership = serde_json::json!({ "in_playlist": id });
                (
                    rule::build_rule_query(&conn, &membership, None)?,
                    order_clause(row.sort_rule.as_ref()),
                )
            };

            let mut stmt = conn
                .prepare(&format!(
                    "SELECT {columns} FROM tracks t
                     WHERE t.trashed_at IS NULL AND ({})
                     ORDER BY {order}",
                    query.where_sql
                ))
                .map_err(err)?;
            let tracks = stmt
                .query_map(params_from_iter(query.params.iter()), map_track_row)
                .map_err(err)?
                .collect::<rusqlite::Result<Vec<_>>>()
                .map_err(err)?;
            Ok(tracks
                .into_iter()
                .enumerate()
                .map(|(index, track)| PlaylistEntry {
                    entry_id: None,
                    position: index as i64,
                    track: library.expand_track(track),
                })
                .collect())
        }
    }
}

// ------------------------------------------------------------ 列としての操作

fn ensure_static(conn: &Connection, id: i64) -> Result<(), String> {
    let row = load_row(conn, id)?;
    if row.kind != PlaylistKind::Static {
        return Err("曲を直接入れられるのは静的プレイリストだけです".to_string());
    }
    Ok(())
}

/// 曲を列に足す。既に入っている曲でも黙って足す。列なので重複は正当である。
/// 返すのは作った要素の ID で、並びは渡された `track_ids` と同じ。
pub fn add_tracks(
    library: &LibraryState,
    playlist_id: i64,
    track_ids: &[i64],
    position: Option<i64>,
) -> Result<Vec<i64>, String> {
    if track_ids.is_empty() {
        return Ok(Vec::new());
    }
    let mut guard = library.conn().map_err(err)?;
    let tx = guard.transaction().map_err(err)?;
    ensure_static(&tx, playlist_id)?;

    let length: i64 = tx
        .query_row(
            "SELECT COUNT(*) FROM playlist_entries WHERE playlist_id = ?1",
            params![playlist_id],
            |row| row.get(0),
        )
        .map_err(err)?;
    let at = position.unwrap_or(length).clamp(0, length);

    // 挿入位置より後ろを空ける。振り直しは最後にまとめてやる。
    tx.execute(
        "UPDATE playlist_entries SET position = position + ?3
         WHERE playlist_id = ?1 AND position >= ?2",
        params![playlist_id, at, track_ids.len() as i64],
    )
    .map_err(err)?;

    let stamp = now();
    let mut created = Vec::with_capacity(track_ids.len());
    for (offset, track_id) in track_ids.iter().enumerate() {
        let exists: bool = tx
            .query_row(
                "SELECT 1 FROM tracks WHERE id = ?1",
                params![track_id],
                |_| Ok(true),
            )
            .optional()
            .map_err(err)?
            .unwrap_or(false);
        if !exists {
            return Err(format!("曲が見つかりません: {track_id}"));
        }
        tx.execute(
            "INSERT INTO playlist_entries (playlist_id, track_id, position, added_at)
             VALUES (?1, ?2, ?3, ?4)",
            params![playlist_id, track_id, at + offset as i64, stamp],
        )
        .map_err(err)?;
        created.push(tx.last_insert_rowid());
    }

    renumber_entries(&tx, playlist_id)?;
    touch(&tx, playlist_id)?;
    tx.commit().map_err(err)?;
    Ok(created)
}

/// この要素だけ消す。同じ曲の別の要素は残る。
pub fn remove_entries(
    library: &LibraryState,
    playlist_id: i64,
    entry_ids: &[i64],
) -> Result<u32, String> {
    if entry_ids.is_empty() {
        return Ok(0);
    }
    let mut guard = library.conn().map_err(err)?;
    let tx = guard.transaction().map_err(err)?;
    ensure_static(&tx, playlist_id)?;

    let placeholders = vec!["?"; entry_ids.len()].join(", ");
    let mut values: Vec<i64> = vec![playlist_id];
    values.extend_from_slice(entry_ids);
    let removed = tx
        .execute(
            &format!(
                "DELETE FROM playlist_entries WHERE playlist_id = ?1 AND id IN ({placeholders})"
            ),
            params_from_iter(values.iter()),
        )
        .map_err(err)?;

    renumber_entries(&tx, playlist_id)?;
    touch(&tx, playlist_id)?;
    tx.commit().map_err(err)?;
    Ok(removed as u32)
}

/// 同じ曲の要素を全部消す。
pub fn remove_tracks(
    library: &LibraryState,
    playlist_id: i64,
    track_ids: &[i64],
) -> Result<u32, String> {
    if track_ids.is_empty() {
        return Ok(0);
    }
    let mut guard = library.conn().map_err(err)?;
    let tx = guard.transaction().map_err(err)?;
    ensure_static(&tx, playlist_id)?;

    let placeholders = vec!["?"; track_ids.len()].join(", ");
    let mut values: Vec<i64> = vec![playlist_id];
    values.extend_from_slice(track_ids);
    let removed = tx
        .execute(
            &format!(
                "DELETE FROM playlist_entries
                 WHERE playlist_id = ?1 AND track_id IN ({placeholders})"
            ),
            params_from_iter(values.iter()),
        )
        .map_err(err)?;

    renumber_entries(&tx, playlist_id)?;
    touch(&tx, playlist_id)?;
    tx.commit().map_err(err)?;
    Ok(removed as u32)
}

/// 列を並べ替える。要素 ID の**完全な**配列を受ける。
/// トラック ID では同じ曲の2つの要素を区別できないので、ここは要素 ID しか受けない。
pub fn reorder_entries(
    library: &LibraryState,
    playlist_id: i64,
    entry_ids: &[i64],
) -> Result<(), String> {
    let mut guard = library.conn().map_err(err)?;
    let tx = guard.transaction().map_err(err)?;
    ensure_static(&tx, playlist_id)?;

    let mut current: Vec<i64> = {
        let mut stmt = tx
            .prepare("SELECT id FROM playlist_entries WHERE playlist_id = ?1")
            .map_err(err)?;
        let ids = stmt
            .query_map(params![playlist_id], |row| row.get(0))
            .map_err(err)?
            .collect::<rusqlite::Result<Vec<_>>>()
            .map_err(err)?;
        ids
    };
    let mut given = entry_ids.to_vec();
    current.sort_unstable();
    given.sort_unstable();
    if current != given {
        return Err("並べ替えには列の要素をすべて渡す必要があります".to_string());
    }

    for (index, entry_id) in entry_ids.iter().enumerate() {
        tx.execute(
            "UPDATE playlist_entries SET position = ?2 WHERE id = ?1",
            params![entry_id, index as i64],
        )
        .map_err(err)?;
    }
    touch(&tx, playlist_id)?;
    tx.commit().map_err(err)?;
    Ok(())
}

fn touch(conn: &Connection, playlist_id: i64) -> Result<(), String> {
    conn.execute(
        "UPDATE playlists SET updated_at = ?2 WHERE id = ?1",
        params![playlist_id, now()],
    )
    .map_err(err)?;
    Ok(())
}

/// 重複解決で負けた側の所属を勝ち側へ引き継ぐ。
/// 列の中では**畳まない**。位置も出現回数もそのまま残す。2回鳴る曲は2回鳴るままである。
pub fn carry_over(
    library: &LibraryState,
    from_track_id: i64,
    to_track_id: i64,
) -> Result<(), String> {
    if from_track_id == to_track_id {
        return Ok(());
    }
    let mut guard = library.conn().map_err(err)?;
    let tx = guard.transaction().map_err(err)?;
    tx.execute(
        "UPDATE playlist_entries SET track_id = ?2 WHERE track_id = ?1",
        params![from_track_id, to_track_id],
    )
    .map_err(err)?;
    tag::carry_over_tags(&tx, from_track_id, to_track_id)?;
    tx.commit().map_err(err)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::library::db::NewTrack;
    use serde_json::json;
    use uuid::Uuid;

    fn temp_library() -> LibraryState {
        let root = std::env::temp_dir().join(format!("catra-playlist-{}", Uuid::new_v4()));
        std::fs::create_dir_all(&root).unwrap();
        LibraryState::new(root).unwrap()
    }

    fn insert_track(library: &LibraryState, title: &str) -> i64 {
        library
            .insert_track(NewTrack {
                path: &format!("library/{}.wav", Uuid::new_v4()),
                title: Some(title),
                artist: Some("Artist"),
                album: Some("Album"),
                duration_ms: Some(1000),
                bpm: Some(128.0),
                bitrate_kbps: None,
                genre: Some("House"),
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

    fn positions(library: &LibraryState, playlist_id: i64) -> Vec<i64> {
        entries(library, playlist_id)
            .unwrap()
            .into_iter()
            .map(|entry| entry.position)
            .collect()
    }

    #[test]
    fn the_same_song_can_enter_a_sequence_twice() {
        let library = temp_library();
        let mix = create(&library, "DJ Mix", None, PlaylistKind::Static).unwrap();
        let acapella = insert_track(&library, "Acapella");
        let filler = insert_track(&library, "Tool");

        let first = add_tracks(&library, mix.id, &[acapella, filler, acapella], None).unwrap();
        assert_eq!(first.len(), 3);
        assert_ne!(first[0], first[2]);

        let list = entries(&library, mix.id).unwrap();
        assert_eq!(list.len(), 3);
        assert_eq!(list[0].track.id, acapella);
        assert_eq!(list[2].track.id, acapella);
        assert_eq!(
            node_by_id(&library.conn().unwrap(), mix.id)
                .unwrap()
                .track_count,
            3
        );
    }

    #[test]
    fn removing_one_entry_leaves_the_other_occurrence() {
        let library = temp_library();
        let mix = create(&library, "DJ Mix", None, PlaylistKind::Static).unwrap();
        let song = insert_track(&library, "Reprise");
        let ids = add_tracks(&library, mix.id, &[song, song], None).unwrap();

        assert_eq!(remove_entries(&library, mix.id, &[ids[0]]).unwrap(), 1);
        let list = entries(&library, mix.id).unwrap();
        assert_eq!(list.len(), 1);
        assert_eq!(list[0].entry_id, Some(ids[1]));
        assert_eq!(list[0].position, 0);
    }

    #[test]
    fn removing_by_track_takes_every_occurrence() {
        let library = temp_library();
        let mix = create(&library, "DJ Mix", None, PlaylistKind::Static).unwrap();
        let song = insert_track(&library, "Reprise");
        let other = insert_track(&library, "Other");
        add_tracks(&library, mix.id, &[song, other, song], None).unwrap();

        assert_eq!(remove_tracks(&library, mix.id, &[song]).unwrap(), 2);
        assert_eq!(entries(&library, mix.id).unwrap().len(), 1);
    }

    #[test]
    fn positions_stay_gapless_through_insert_and_delete() {
        let library = temp_library();
        let mix = create(&library, "Set", None, PlaylistKind::Static).unwrap();
        let tracks: Vec<i64> = (0..4)
            .map(|i| insert_track(&library, &format!("t{i}")))
            .collect();
        let ids = add_tracks(&library, mix.id, &tracks, None).unwrap();
        assert_eq!(positions(&library, mix.id), vec![0, 1, 2, 3]);

        let extra = insert_track(&library, "middle");
        add_tracks(&library, mix.id, &[extra], Some(1)).unwrap();
        assert_eq!(positions(&library, mix.id), vec![0, 1, 2, 3, 4]);
        assert_eq!(entries(&library, mix.id).unwrap()[1].track.id, extra);

        remove_entries(&library, mix.id, &[ids[0], ids[2]]).unwrap();
        assert_eq!(positions(&library, mix.id), vec![0, 1, 2]);
    }

    #[test]
    fn two_entries_of_one_song_move_independently() {
        let library = temp_library();
        let mix = create(&library, "Set", None, PlaylistKind::Static).unwrap();
        let song = insert_track(&library, "Twice");
        let other = insert_track(&library, "Other");
        let ids = add_tracks(&library, mix.id, &[song, other, song], None).unwrap();

        // 2番目の出現だけを先頭へ動かす。
        reorder_entries(&library, mix.id, &[ids[2], ids[0], ids[1]]).unwrap();
        let list = entries(&library, mix.id).unwrap();
        assert_eq!(
            list.iter().map(|e| e.entry_id.unwrap()).collect::<Vec<_>>(),
            vec![ids[2], ids[0], ids[1]]
        );
        assert_eq!(positions(&library, mix.id), vec![0, 1, 2]);
    }

    #[test]
    fn reordering_preserves_the_number_of_occurrences() {
        let library = temp_library();
        let mix = create(&library, "Set", None, PlaylistKind::Static).unwrap();
        let song = insert_track(&library, "Twice");
        let ids = add_tracks(&library, mix.id, &[song, song, song], None).unwrap();
        reorder_entries(&library, mix.id, &[ids[1], ids[2], ids[0]]).unwrap();
        let list = entries(&library, mix.id).unwrap();
        assert_eq!(list.iter().filter(|e| e.track.id == song).count(), 3);
    }

    #[test]
    fn reordering_rejects_a_partial_list() {
        let library = temp_library();
        let mix = create(&library, "Set", None, PlaylistKind::Static).unwrap();
        let song = insert_track(&library, "One");
        let ids = add_tracks(&library, mix.id, &[song, song], None).unwrap();
        assert!(reorder_entries(&library, mix.id, &[ids[0]]).is_err());
    }

    #[test]
    fn a_folder_cannot_be_moved_into_its_own_descendant() {
        let library = temp_library();
        let parent = create(&library, "Parent", None, PlaylistKind::Folder).unwrap();
        let child = create(&library, "Child", Some(parent.id), PlaylistKind::Folder).unwrap();
        assert!(move_node(&library, parent.id, Some(child.id), 0).is_err());
        assert!(move_node(&library, parent.id, Some(parent.id), 0).is_err());
    }

    #[test]
    fn a_playlist_cannot_live_inside_a_playlist() {
        let library = temp_library();
        let list = create(&library, "Set", None, PlaylistKind::Static).unwrap();
        assert!(create(&library, "Nested", Some(list.id), PlaylistKind::Static).is_err());
    }

    #[test]
    fn moving_renumbers_both_sides() {
        let library = temp_library();
        let a = create(&library, "A", None, PlaylistKind::Folder).unwrap();
        let b = create(&library, "B", None, PlaylistKind::Folder).unwrap();
        let inside = create(&library, "Inside", Some(a.id), PlaylistKind::Static).unwrap();
        let sibling = create(&library, "Sibling", Some(a.id), PlaylistKind::Static).unwrap();

        move_node(&library, inside.id, Some(b.id), 0).unwrap();
        let tree = list_tree(&library).unwrap();
        let moved = tree.iter().find(|node| node.id == inside.id).unwrap();
        assert_eq!(moved.parent_id, Some(b.id));
        assert_eq!(moved.position, 0);
        let left = tree.iter().find(|node| node.id == sibling.id).unwrap();
        assert_eq!(left.position, 0);
    }

    #[test]
    fn deleting_a_folder_takes_its_subtree_but_not_the_songs() {
        let library = temp_library();
        let folder = create(&library, "Folder", None, PlaylistKind::Folder).unwrap();
        let inner = create(&library, "Inner", Some(folder.id), PlaylistKind::Static).unwrap();
        let song = insert_track(&library, "Song");
        add_tracks(&library, inner.id, &[song], None).unwrap();

        assert_eq!(delete(&library, folder.id).unwrap(), 2);
        assert!(list_tree(&library).unwrap().is_empty());
        assert!(library.get_track(song).unwrap().is_some());
    }

    #[test]
    fn a_folder_is_the_union_of_its_descendants_with_duplicates_collapsed() {
        let library = temp_library();
        let folder = create(&library, "Folder", None, PlaylistKind::Folder).unwrap();
        let left = create(&library, "Left", Some(folder.id), PlaylistKind::Static).unwrap();
        let right = create(&library, "Right", Some(folder.id), PlaylistKind::Static).unwrap();
        let shared = insert_track(&library, "Shared");
        let only = insert_track(&library, "Only");
        add_tracks(&library, left.id, &[shared, shared], None).unwrap();
        add_tracks(&library, right.id, &[shared, only], None).unwrap();

        let union = entries(&library, folder.id).unwrap();
        assert_eq!(union.len(), 2);
        assert!(union.iter().all(|entry| entry.entry_id.is_none()));
        assert_eq!(
            node_by_id(&library.conn().unwrap(), folder.id)
                .unwrap()
                .track_count,
            2
        );
    }

    #[test]
    fn a_smart_playlist_is_a_set_and_hides_its_entry_ids() {
        let library = temp_library();
        let smart = create(&library, "House", None, PlaylistKind::Smart).unwrap();
        insert_track(&library, "One");
        insert_track(&library, "Two");
        set_rule(
            &library,
            smart.id,
            Some(json!({ "field": "genre", "op": "eq", "value": "house" })),
            None,
        )
        .unwrap();

        let list = entries(&library, smart.id).unwrap();
        assert_eq!(list.len(), 2);
        assert!(list.iter().all(|entry| entry.entry_id.is_none()));
    }

    #[test]
    fn a_rule_that_reaches_itself_is_rejected() {
        let library = temp_library();
        let smart = create(&library, "Loop", None, PlaylistKind::Smart).unwrap();
        assert!(set_rule(
            &library,
            smart.id,
            Some(json!({ "in_playlist": smart.id })),
            None
        )
        .is_err());
    }

    #[test]
    fn deleting_a_referenced_playlist_prunes_the_rule() {
        let library = temp_library();
        let source = create(&library, "Source", None, PlaylistKind::Static).unwrap();
        let smart = create(&library, "Derived", None, PlaylistKind::Smart).unwrap();
        set_rule(
            &library,
            smart.id,
            Some(json!({ "all": [
                { "in_playlist": source.id },
                { "field": "bpm", "op": "gte", "value": 120 }
            ]})),
            None,
        )
        .unwrap();

        let impact = delete_impact(&library, source.id).unwrap();
        assert_eq!(impact.referencing.len(), 1);
        assert_eq!(impact.referencing[0].id, smart.id);

        delete(&library, source.id).unwrap();
        let after = node_by_id(&library.conn().unwrap(), smart.id).unwrap();
        let kept = after.rule.unwrap();
        let terms = kept["all"].as_array().unwrap();
        assert_eq!(terms.len(), 1);
        assert_eq!(terms[0]["field"], "bpm");
    }

    #[test]
    fn trashed_songs_drop_out_of_the_sequence_without_losing_their_entry() {
        let library = temp_library();
        let mix = create(&library, "Set", None, PlaylistKind::Static).unwrap();
        let song = insert_track(&library, "Song");
        add_tracks(&library, mix.id, &[song], None).unwrap();

        library
            .conn()
            .unwrap()
            .execute(
                "UPDATE tracks SET trashed_at = 1 WHERE id = ?1",
                params![song],
            )
            .unwrap();
        assert!(entries(&library, mix.id).unwrap().is_empty());

        library
            .conn()
            .unwrap()
            .execute(
                "UPDATE tracks SET trashed_at = NULL WHERE id = ?1",
                params![song],
            )
            .unwrap();
        assert_eq!(entries(&library, mix.id).unwrap().len(), 1);
    }

    #[test]
    fn carry_over_keeps_every_occurrence_and_its_place() {
        let library = temp_library();
        let mix = create(&library, "Set", None, PlaylistKind::Static).unwrap();
        let old = insert_track(&library, "Old");
        let other = insert_track(&library, "Other");
        let new = insert_track(&library, "New");
        add_tracks(&library, mix.id, &[old, other, old], None).unwrap();

        carry_over(&library, old, new).unwrap();
        let list = entries(&library, mix.id).unwrap();
        assert_eq!(list.len(), 3);
        assert_eq!(list[0].track.id, new);
        assert_eq!(list[1].track.id, other);
        assert_eq!(list[2].track.id, new);
        assert_eq!(positions(&library, mix.id), vec![0, 1, 2]);
    }

    #[test]
    fn deleting_a_track_for_good_takes_its_entries() {
        let library = temp_library();
        let mix = create(&library, "Set", None, PlaylistKind::Static).unwrap();
        let song = insert_track(&library, "Song");
        let other = insert_track(&library, "Other");
        add_tracks(&library, mix.id, &[song, other, song], None).unwrap();

        library
            .conn()
            .unwrap()
            .execute("DELETE FROM tracks WHERE id = ?1", params![song])
            .unwrap();
        let left: i64 = library
            .conn()
            .unwrap()
            .query_row(
                "SELECT COUNT(*) FROM playlist_entries WHERE playlist_id = ?1",
                params![mix.id],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(left, 1);
    }
}
