//! スマートプレイリストの述語。JSON を受け取り、`tracks` への WHERE 句に組み立てる。
//! SQL 文字列は保存しない。保存するのは常に JSON である。

use rusqlite::types::Value as SqlValue;
use rusqlite::{params, Connection, OptionalExtension};
use serde_json::Value;

/// 入れ子の上限。壊れた JSON で無限に潜らないための保険。
const MAX_DEPTH: usize = 12;

#[derive(Debug)]
pub struct RuleQuery {
    pub where_sql: String,
    pub params: Vec<SqlValue>,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum FieldType {
    Text,
    Number,
    Bool,
}

/// 述語に出せるフィールド。`tracks` にリリース年・レーベル・コメント・リミキサーの列がないため、
/// それらはまだ出さない（`classification.md` の「前提: 足りない列」）。
fn column_for(field: &str) -> Option<(&'static str, FieldType)> {
    match field {
        "title" => Some(("title", FieldType::Text)),
        "artist" => Some(("artist", FieldType::Text)),
        "album" => Some(("album", FieldType::Text)),
        "genre" => Some(("genre", FieldType::Text)),
        "key" => Some(("key_name", FieldType::Text)),
        "source" => Some(("source", FieldType::Text)),
        "path" => Some(("path", FieldType::Text)),
        "bpm" => Some(("bpm", FieldType::Number)),
        "rating" => Some(("rating", FieldType::Number)),
        "bitrateKbps" => Some(("bitrate_kbps", FieldType::Number)),
        "durationMs" => Some(("duration_ms", FieldType::Number)),
        "addedAt" => Some(("added_at", FieldType::Number)),
        "converted" => Some(("converted", FieldType::Bool)),
        _ => None,
    }
}

/// 並べ替えの列名を引く。述語に出せるフィールドと同じ集合に揃える。
pub fn column_name(field: &str) -> Option<&'static str> {
    column_for(field).map(|(column, _)| column)
}

/// 述語をゴミ箱を除いたトラック集合の WHERE 句にする。
/// `self_id` を渡すと、そのプレイリスト自身への到達を循環として弾く。
pub fn build_rule_query(
    conn: &Connection,
    rule: &Value,
    self_id: Option<i64>,
) -> Result<RuleQuery, String> {
    let mut out = Vec::new();
    let mut visiting: Vec<i64> = self_id.into_iter().collect();
    let where_sql = build_term(conn, rule, "t", &mut out, &mut visiting, 0)?;
    Ok(RuleQuery {
        where_sql,
        params: out,
    })
}

fn build_term(
    conn: &Connection,
    term: &Value,
    alias: &str,
    out: &mut Vec<SqlValue>,
    visiting: &mut Vec<i64>,
    depth: usize,
) -> Result<String, String> {
    if depth > MAX_DEPTH {
        return Err("述語の入れ子が深すぎます".to_string());
    }
    let Some(object) = term.as_object() else {
        return Err("述語の項はオブジェクトである必要があります".to_string());
    };

    if let Some(children) = object.get("all") {
        return build_group(conn, children, alias, out, visiting, depth, "AND", "1");
    }
    if let Some(children) = object.get("any") {
        return build_group(conn, children, alias, out, visiting, depth, "OR", "0");
    }
    if let Some(child) = object.get("not") {
        let inner = build_term(conn, child, alias, out, visiting, depth + 1)?;
        return Ok(format!("NOT ({inner})"));
    }
    if let Some(tag) = object.get("tag") {
        let tag_id = tag
            .as_i64()
            .ok_or_else(|| "tag の項はタグ ID を指す必要があります".to_string())?;
        out.push(SqlValue::Integer(tag_id));
        return Ok(format!(
            "{alias}.id IN (SELECT track_id FROM track_tags WHERE tag_id = ?)"
        ));
    }
    if let Some(playlist) = object.get("in_playlist") {
        let playlist_id = playlist
            .as_i64()
            .ok_or_else(|| "in_playlist の項はプレイリスト ID を指す必要があります".to_string())?;
        return build_membership(conn, playlist_id, alias, out, visiting, depth);
    }
    if object.contains_key("field") {
        return build_field(object, alias, out);
    }

    Err("述語の項の種類が分かりません".to_string())
}

fn build_group(
    conn: &Connection,
    children: &Value,
    alias: &str,
    out: &mut Vec<SqlValue>,
    visiting: &mut Vec<i64>,
    depth: usize,
    joiner: &str,
    empty: &str,
) -> Result<String, String> {
    let Some(items) = children.as_array() else {
        return Err("all / any の中身は配列である必要があります".to_string());
    };
    if items.is_empty() {
        return Ok(empty.to_string());
    }
    let mut parts = Vec::with_capacity(items.len());
    for item in items {
        parts.push(build_term(conn, item, alias, out, visiting, depth + 1)?);
    }
    Ok(format!("({})", parts.join(&format!(" {joiner} "))))
}

/// 集合への所属判定。静的プレイリストが列でも、ここが見るのは「現れるか」の真偽だけである。
fn build_membership(
    conn: &Connection,
    playlist_id: i64,
    alias: &str,
    out: &mut Vec<SqlValue>,
    visiting: &mut Vec<i64>,
    depth: usize,
) -> Result<String, String> {
    if visiting.contains(&playlist_id) {
        return Err("述語が循環しています".to_string());
    }

    let row: Option<(String, Option<String>)> = conn
        .query_row(
            "SELECT kind, rule FROM playlists WHERE id = ?1",
            params![playlist_id],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .optional()
        .map_err(|error| error.to_string())?;

    // 参照先が消えている項は偽として扱う。削除時の刈り取りが追いつく前でも評価は止めない。
    let Some((kind, rule)) = row else {
        return Ok("0".to_string());
    };

    visiting.push(playlist_id);
    let result = match kind.as_str() {
        "static" => {
            out.push(SqlValue::Integer(playlist_id));
            Ok(format!(
                "{alias}.id IN (SELECT track_id FROM playlist_entries WHERE playlist_id = ?)"
            ))
        }
        "smart" => match rule {
            Some(raw) => {
                let parsed: Value = serde_json::from_str(&raw)
                    .map_err(|error| format!("規則の JSON が壊れています: {error}"))?;
                let inner_alias = format!("t{}", depth + 1);
                let inner = build_term(conn, &parsed, &inner_alias, out, visiting, depth + 1)?;
                Ok(format!(
                    "{alias}.id IN (SELECT {inner_alias}.id FROM tracks {inner_alias}
                     WHERE {inner_alias}.trashed_at IS NULL AND ({inner}))"
                ))
            }
            None => Ok("0".to_string()),
        },
        "folder" => {
            let children = child_ids(conn, playlist_id).map_err(|error| error.to_string())?;
            if children.is_empty() {
                Ok("0".to_string())
            } else {
                let mut parts = Vec::with_capacity(children.len());
                for child in children {
                    parts.push(build_membership(
                        conn,
                        child,
                        alias,
                        out,
                        visiting,
                        depth + 1,
                    )?);
                }
                Ok(format!("({})", parts.join(" OR ")))
            }
        }
        other => Err(format!("プレイリストの種類が不正です: {other}")),
    };
    visiting.pop();
    result
}

fn child_ids(conn: &Connection, parent_id: i64) -> rusqlite::Result<Vec<i64>> {
    let mut stmt =
        conn.prepare("SELECT id FROM playlists WHERE parent_id = ?1 ORDER BY position")?;
    let ids = stmt
        .query_map(params![parent_id], |row| row.get(0))?
        .collect();
    ids
}

fn build_field(
    object: &serde_json::Map<String, Value>,
    alias: &str,
    out: &mut Vec<SqlValue>,
) -> Result<String, String> {
    let field = object
        .get("field")
        .and_then(Value::as_str)
        .ok_or_else(|| "field の項はフィールド名が要ります".to_string())?;
    let (column, kind) =
        column_for(field).ok_or_else(|| format!("述語に使えないフィールドです: {field}"))?;
    let op = object.get("op").and_then(Value::as_str).unwrap_or("eq");
    let value = object.get("value").unwrap_or(&Value::Null);
    let target = format!("{alias}.{column}");

    match op {
        "isNull" => {
            return Ok(match kind {
                FieldType::Text => format!("({target} IS NULL OR trim({target}) = '')"),
                _ => format!("{target} IS NULL"),
            })
        }
        "isNotNull" => {
            return Ok(match kind {
                FieldType::Text => format!("({target} IS NOT NULL AND trim({target}) != '')"),
                _ => format!("{target} IS NOT NULL"),
            })
        }
        "between" => {
            let bounds = value
                .as_array()
                .filter(|items| items.len() == 2)
                .ok_or_else(|| "between の値は2要素の配列が要ります".to_string())?;
            out.push(number_value(&bounds[0])?);
            out.push(number_value(&bounds[1])?);
            return Ok(format!("{target} BETWEEN ? AND ?"));
        }
        _ => {}
    }

    match kind {
        FieldType::Text => {
            let text = value
                .as_str()
                .ok_or_else(|| format!("{field} には文字列が要ります"))?;
            match op {
                "eq" => {
                    out.push(SqlValue::Text(text.to_string()));
                    Ok(format!("{target} = ? COLLATE NOCASE"))
                }
                "ne" => {
                    out.push(SqlValue::Text(text.to_string()));
                    Ok(format!(
                        "({target} IS NULL OR {target} != ? COLLATE NOCASE)"
                    ))
                }
                "contains" => like(target, format!("%{}%", escape_like(text)), out),
                "startsWith" => like(target, format!("{}%", escape_like(text)), out),
                "endsWith" => like(target, format!("%{}", escape_like(text)), out),
                other => Err(format!("文字列に使えない演算子です: {other}")),
            }
        }
        FieldType::Number => {
            let number = number_value(value)?;
            out.push(number);
            let operator = match op {
                "eq" => "=",
                "ne" => "!=",
                "gt" => ">",
                "gte" => ">=",
                "lt" => "<",
                "lte" => "<=",
                other => return Err(format!("数値に使えない演算子です: {other}")),
            };
            Ok(format!("{target} {operator} ?"))
        }
        FieldType::Bool => {
            let flag = value
                .as_bool()
                .ok_or_else(|| format!("{field} には真偽値が要ります"))?;
            out.push(SqlValue::Integer(flag as i64));
            Ok(format!("{target} = ?"))
        }
    }
}

fn like(target: String, pattern: String, out: &mut Vec<SqlValue>) -> Result<String, String> {
    out.push(SqlValue::Text(pattern));
    Ok(format!("{target} LIKE ? ESCAPE '\\'"))
}

fn escape_like(value: &str) -> String {
    value
        .replace('\\', "\\\\")
        .replace('%', "\\%")
        .replace('_', "\\_")
}

fn number_value(value: &Value) -> Result<SqlValue, String> {
    if let Some(integer) = value.as_i64() {
        return Ok(SqlValue::Integer(integer));
    }
    if let Some(float) = value.as_f64() {
        return Ok(SqlValue::Real(float));
    }
    Err("数値が要ります".to_string())
}

/// 述語が参照しているタグ ID とプレイリスト ID を集める。参照先を消したときの刈り取りに使う。
pub fn collect_refs(rule: &Value, tags: &mut Vec<i64>, playlists: &mut Vec<i64>) {
    match rule {
        Value::Object(object) => {
            if let Some(id) = object.get("tag").and_then(Value::as_i64) {
                tags.push(id);
            }
            if let Some(id) = object.get("in_playlist").and_then(Value::as_i64) {
                playlists.push(id);
            }
            for child in object.values() {
                collect_refs(child, tags, playlists);
            }
        }
        Value::Array(items) => {
            for item in items {
                collect_refs(item, tags, playlists);
            }
        }
        _ => {}
    }
}

/// 消えたタグ／プレイリストを指す項を落とす。落とした結果 `all` / `any` が空になっても、
/// 空の `all` は真、空の `any` は偽として評価されるのでそのまま残す。
/// 述語の全体が落ちるときは `None` を返す（規則なしのスマートプレイリストになる）。
pub fn prune_refs(rule: &Value, dead_tags: &[i64], dead_playlists: &[i64]) -> Option<Value> {
    let object = rule.as_object()?;

    if let Some(id) = object.get("tag").and_then(Value::as_i64) {
        if dead_tags.contains(&id) {
            return None;
        }
        return Some(rule.clone());
    }
    if let Some(id) = object.get("in_playlist").and_then(Value::as_i64) {
        if dead_playlists.contains(&id) {
            return None;
        }
        return Some(rule.clone());
    }
    if let Some(child) = object.get("not") {
        let pruned = prune_refs(child, dead_tags, dead_playlists)?;
        return Some(serde_json::json!({ "not": pruned }));
    }
    for key in ["all", "any"] {
        if let Some(Value::Array(items)) = object.get(key) {
            let kept: Vec<Value> = items
                .iter()
                .filter_map(|item| prune_refs(item, dead_tags, dead_playlists))
                .collect();
            return Some(serde_json::json!({ key: kept }));
        }
    }

    Some(rule.clone())
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn memory_db() -> Connection {
        let conn = Connection::open_in_memory().unwrap();
        conn.execute_batch(
            "CREATE TABLE playlists (id INTEGER PRIMARY KEY, parent_id INTEGER, kind TEXT,
                 name TEXT, position INTEGER, rule TEXT, sort_rule TEXT);
             CREATE TABLE playlist_entries (id INTEGER PRIMARY KEY, playlist_id INTEGER,
                 track_id INTEGER, position INTEGER, added_at INTEGER);
             CREATE TABLE track_tags (track_id INTEGER, tag_id INTEGER, assigned_at INTEGER);",
        )
        .unwrap();
        conn
    }

    #[test]
    fn field_terms_bind_values_instead_of_inlining_them() {
        let conn = memory_db();
        let query = build_rule_query(
            &conn,
            &json!({ "field": "genre", "op": "contains", "value": "house" }),
            None,
        )
        .unwrap();
        assert!(query.where_sql.contains("LIKE ?"));
        assert_eq!(query.params.len(), 1);
        assert_eq!(query.params[0], SqlValue::Text("%house%".to_string()));
    }

    #[test]
    fn like_wildcards_in_user_input_are_escaped() {
        let conn = memory_db();
        let query = build_rule_query(
            &conn,
            &json!({ "field": "title", "op": "contains", "value": "100%_mix" }),
            None,
        )
        .unwrap();
        assert_eq!(
            query.params[0],
            SqlValue::Text("%100\\%\\_mix%".to_string())
        );
    }

    #[test]
    fn unknown_field_is_rejected() {
        let conn = memory_db();
        let error = build_rule_query(
            &conn,
            &json!({ "field": "releaseYear", "op": "eq", "value": 2024 }),
            None,
        )
        .unwrap_err();
        assert!(error.contains("releaseYear"));
    }

    #[test]
    fn membership_in_a_sequence_is_a_set_test() {
        let conn = memory_db();
        conn.execute(
            "INSERT INTO playlists (id, kind, name, position) VALUES (1, 'static', 'mix', 0)",
            [],
        )
        .unwrap();
        let query = build_rule_query(&conn, &json!({ "in_playlist": 1 }), None).unwrap();
        // 同じ曲が何度現れても IN は1回しか真偽を返さない。
        assert!(query
            .where_sql
            .contains("IN (SELECT track_id FROM playlist_entries"));
    }

    #[test]
    fn self_reference_is_a_cycle() {
        let conn = memory_db();
        conn.execute(
            "INSERT INTO playlists (id, kind, name, position) VALUES (5, 'static', 'a', 0)",
            [],
        )
        .unwrap();
        let error = build_rule_query(&conn, &json!({ "in_playlist": 5 }), Some(5)).unwrap_err();
        assert!(error.contains("循環"));
    }

    #[test]
    fn mutual_reference_between_smart_playlists_is_a_cycle() {
        let conn = memory_db();
        conn.execute(
            "INSERT INTO playlists (id, kind, name, position, rule)
             VALUES (1, 'smart', 'a', 0, '{\"in_playlist\": 2}')",
            [],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO playlists (id, kind, name, position, rule)
             VALUES (2, 'smart', 'b', 0, '{\"in_playlist\": 1}')",
            [],
        )
        .unwrap();
        let error = build_rule_query(&conn, &json!({ "in_playlist": 2 }), Some(1)).unwrap_err();
        assert!(error.contains("循環"));
    }

    #[test]
    fn missing_reference_evaluates_to_false() {
        let conn = memory_db();
        let query = build_rule_query(&conn, &json!({ "in_playlist": 99 }), None).unwrap();
        assert_eq!(query.where_sql, "0");
    }

    #[test]
    fn pruning_drops_only_the_dead_term() {
        let rule = json!({ "all": [
            { "tag": 7 },
            { "field": "bpm", "op": "between", "value": [128, 132] },
            { "not": { "in_playlist": 12 } }
        ]});
        let pruned = prune_refs(&rule, &[7], &[12]).unwrap();
        let kept = pruned["all"].as_array().unwrap();
        assert_eq!(kept.len(), 1);
        assert_eq!(kept[0]["field"], "bpm");
    }

    #[test]
    fn collect_refs_finds_nested_ids() {
        let rule = json!({ "any": [{ "tag": 3 }, { "not": { "in_playlist": 9 } }] });
        let mut tags = Vec::new();
        let mut playlists = Vec::new();
        collect_refs(&rule, &mut tags, &mut playlists);
        assert_eq!(tags, vec![3]);
        assert_eq!(playlists, vec![9]);
    }
}
