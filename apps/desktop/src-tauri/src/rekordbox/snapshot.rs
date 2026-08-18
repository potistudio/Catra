use super::content::{purge_content_rows, RekordboxContent};
use super::db::MasterDatabase;
use super::write::WriteSession;
use rusqlite::types::{Value, ValueRef};
use rusqlite::{params, params_from_iter, Connection, OptionalExtension};
use serde_json::{Map, Value as JsonValue};

const SNAPSHOT_TABLES: &[(&str, &str)] = &[
    ("djmdContent", "ID"),
    ("djmdCue", "ContentID"),
    ("djmdSongPlaylist", "ContentID"),
    ("djmdSongMyTag", "ContentID"),
];

const IMPORT_ORDER: &[&str] = &[
    "djmdContent",
    "djmdCue",
    "djmdSongPlaylist",
    "djmdSongMyTag",
];

/// Copy Content, Cue, and playlist membership rows as JSON.
/// Analysis files on disk are not copied; `UUID` and `AnalysisDataPath` stay in the snapshot.
pub fn export_content_snapshot(id: &str) -> Result<String, String> {
    let db = MasterDatabase::open()?;
    if !row_exists(db.conn(), "djmdContent", "ID", id)? {
        return Err(format!("content {id} was not found"));
    }
    let mut tables = Map::new();
    for (table, key) in SNAPSHOT_TABLES {
        if !table_exists(db.conn(), table)? {
            continue;
        }
        tables.insert(
            (*table).to_string(),
            JsonValue::Array(select_rows(db.conn(), table, key, id)?),
        );
    }
    serde_json::to_string(&JsonValue::Object(tables))
        .map_err(|error| format!("failed to serialize content snapshot: {error}"))
}

/// Insert snapshot rows with the original IDs and UUID.
/// When `path` is set, `FolderPath` and `FileNameL` on Content are updated before insert.
pub fn import_content_snapshot(
    snapshot: &str,
    path: Option<&str>,
) -> Result<RekordboxContent, String> {
    let mut tables: Map<String, JsonValue> = serde_json::from_str(snapshot)
        .map_err(|error| format!("invalid content snapshot: {error}"))?;
    let content_id = content_id_from_tables(&tables)?;
    if let Some(path) = path {
        apply_path_to_content(&mut tables, path)?;
    }
    set_json_integer(&mut tables, "djmdContent", "rb_local_deleted", 0);

    let mut session = WriteSession::open()?;
    if row_exists(session.conn(), "djmdContent", "ID", &content_id)? {
        purge_content_rows(&mut session, &content_id)?;
    }
    for table in IMPORT_ORDER {
        let Some(JsonValue::Array(rows)) = tables.get(*table) else {
            continue;
        };
        if rows.is_empty() {
            continue;
        }
        if !table_exists(session.conn(), table)? {
            continue;
        }
        insert_rows(&mut session, table, rows)?;
    }
    let db_dir = session.db_dir.clone();
    session.commit()?;

    let db = MasterDatabase::open()?;
    let mut contents = db.get_content(Some(&content_id), &db_dir)?;
    contents
        .pop()
        .ok_or_else(|| "restored content could not be reloaded".to_string())
}

fn content_id_from_tables(tables: &Map<String, JsonValue>) -> Result<String, String> {
    let row = tables
        .get("djmdContent")
        .and_then(JsonValue::as_array)
        .and_then(|rows| rows.first())
        .and_then(JsonValue::as_object)
        .ok_or_else(|| "content snapshot has no djmdContent row".to_string())?;
    json_id(row.get("ID")).ok_or_else(|| "content snapshot is missing ID".to_string())
}

fn apply_path_to_content(
    tables: &mut Map<String, JsonValue>,
    path: &str,
) -> Result<(), String> {
    let path = std::path::PathBuf::from(path);
    if !path.is_file() {
        return Err(format!("file not found: {}", path.display()));
    }
    let folder_path = path.to_string_lossy().replace('/', "\\");
    let file_name = path
        .file_name()
        .and_then(|value| value.to_str())
        .ok_or_else(|| "invalid file name".to_string())?;
    let Some(JsonValue::Array(rows)) = tables.get_mut("djmdContent") else {
        return Err("content snapshot has no djmdContent row".to_string());
    };
    let Some(JsonValue::Object(row)) = rows.first_mut() else {
        return Err("content snapshot has no djmdContent row".to_string());
    };
    row.insert("FolderPath".to_string(), JsonValue::String(folder_path));
    row.insert("FileNameL".to_string(), JsonValue::String(file_name.to_string()));
    Ok(())
}

fn set_json_integer(tables: &mut Map<String, JsonValue>, table: &str, column: &str, value: i64) {
    let Some(JsonValue::Array(rows)) = tables.get_mut(table) else {
        return;
    };
    for row in rows {
        if let JsonValue::Object(map) = row {
            map.insert(column.to_string(), JsonValue::from(value));
        }
    }
}

fn table_exists(conn: &Connection, table: &str) -> Result<bool, String> {
    conn.query_row(
        "SELECT 1 FROM sqlite_master WHERE type = 'table' AND name = ?1 LIMIT 1",
        params![table],
        |_| Ok(()),
    )
    .optional()
    .map(|row| row.is_some())
    .map_err(|error| error.to_string())
}

fn row_exists(conn: &Connection, table: &str, key: &str, id: &str) -> Result<bool, String> {
    let ident = quote_ident(table)?;
    let key = quote_ident(key)?;
    conn.prepare(&format!("SELECT 1 FROM {ident} WHERE {key} = ?1 LIMIT 1"))
        .map_err(|error| error.to_string())?
        .exists(params![id])
        .map_err(|error| error.to_string())
}

fn select_rows(
    conn: &Connection,
    table: &str,
    key: &str,
    id: &str,
) -> Result<Vec<JsonValue>, String> {
    let ident = quote_ident(table)?;
    let key = quote_ident(key)?;
    let mut stmt = conn
        .prepare(&format!("SELECT * FROM {ident} WHERE {key} = ?1"))
        .map_err(|error| error.to_string())?;
    let names: Vec<String> = stmt
        .column_names()
        .into_iter()
        .map(ToOwned::to_owned)
        .collect();
    let mut rows = stmt
        .query(params![id])
        .map_err(|error| error.to_string())?;
    let mut out = Vec::new();
    while let Some(row) = rows.next().map_err(|error| error.to_string())? {
        let mut map = Map::new();
        for (index, name) in names.iter().enumerate() {
            let value = row.get_ref(index).map_err(|error| error.to_string())?;
            map.insert(name.clone(), sql_to_json(value));
        }
        out.push(JsonValue::Object(map));
    }
    Ok(out)
}

fn insert_rows(
    session: &mut WriteSession,
    table: &str,
    rows: &[JsonValue],
) -> Result<(), String> {
    let ident = quote_ident(table)?;
    for row in rows {
        let JsonValue::Object(map) = row else {
            continue;
        };
        if map.is_empty() {
            continue;
        }
        let columns: Vec<&String> = map.keys().collect();
        let quoted: Result<Vec<String>, String> =
            columns.iter().map(|name| quote_ident(name)).collect();
        let quoted = quoted?;
        let placeholders: Vec<String> = (1..=quoted.len()).map(|i| format!("?{i}")).collect();
        let sql = format!(
            "INSERT INTO {ident} ({}) VALUES ({})",
            quoted.join(", "),
            placeholders.join(", ")
        );
        let values: Vec<Value> = columns
            .iter()
            .map(|name| json_to_sql(map.get(*name).unwrap_or(&JsonValue::Null)))
            .collect();
        session
            .conn()
            .execute(&sql, params_from_iter(values))
            .map_err(|error| format!("failed to restore {table}: {error}"))?;
        if let Some(id) = json_id(map.get("ID")) {
            session.usn.track(static_table(table), id);
        }
    }
    Ok(())
}

fn static_table(table: &str) -> &'static str {
    SNAPSHOT_TABLES
        .iter()
        .map(|(name, _)| *name)
        .find(|name| *name == table)
        .unwrap_or("djmdContent")
}

fn quote_ident(name: &str) -> Result<String, String> {
    if name.is_empty()
        || !name
            .chars()
            .all(|ch| ch.is_ascii_alphanumeric() || ch == '_')
    {
        return Err(format!("invalid identifier: {name}"));
    }
    Ok(format!("\"{name}\""))
}

fn json_id(value: Option<&JsonValue>) -> Option<String> {
    match value? {
        JsonValue::String(text) => Some(text.clone()),
        JsonValue::Number(number) => number.as_i64().map(|n| n.to_string()),
        _ => None,
    }
}

fn sql_to_json(value: ValueRef<'_>) -> JsonValue {
    match value {
        ValueRef::Null => JsonValue::Null,
        ValueRef::Integer(value) => JsonValue::from(value),
        ValueRef::Real(value) => JsonValue::from(value),
        ValueRef::Text(value) => JsonValue::String(String::from_utf8_lossy(value).into_owned()),
        ValueRef::Blob(value) => JsonValue::Array(value.iter().copied().map(JsonValue::from).collect()),
    }
}

fn json_to_sql(value: &JsonValue) -> Value {
    match value {
        JsonValue::Null => Value::Null,
        JsonValue::Bool(value) => Value::Integer(i64::from(*value)),
        JsonValue::Number(value) => {
            if let Some(int) = value.as_i64() {
                Value::Integer(int)
            } else if let Some(uint) = value.as_u64() {
                Value::Integer(uint as i64)
            } else {
                Value::Real(value.as_f64().unwrap_or(0.0))
            }
        }
        JsonValue::String(value) => Value::Text(value.clone()),
        JsonValue::Array(items) => Value::Blob(
            items
                .iter()
                .filter_map(|item| item.as_u64().map(|n| n as u8))
                .collect(),
        ),
        JsonValue::Object(_) => Value::Text(value.to_string()),
    }
}

#[cfg(test)]
mod tests {
    use super::{json_to_sql, quote_ident, sql_to_json, JsonValue};
    use rusqlite::types::{Value, ValueRef};

    #[test]
    fn quote_ident_rejects_injection() {
        assert!(quote_ident("djmdContent").is_ok());
        assert!(quote_ident("FolderPath").is_ok());
        assert!(quote_ident("djmdContent; DROP TABLE").is_err());
        assert!(quote_ident("").is_err());
    }

    #[test]
    fn sql_json_roundtrip_text_and_int() {
        assert_eq!(
            json_to_sql(&sql_to_json(ValueRef::Integer(42))),
            Value::Integer(42)
        );
        assert_eq!(
            json_to_sql(&sql_to_json(ValueRef::Text(b"abc"))),
            Value::Text("abc".to_string())
        );
        assert_eq!(json_to_sql(&JsonValue::Null), Value::Null);
    }
}
