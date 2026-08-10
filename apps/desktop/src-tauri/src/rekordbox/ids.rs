use rusqlite::{params, Connection};
use uuid::Uuid;

/// Generate a unused 28-bit numeric ID as a decimal string for djmd* tables.
pub fn unused_numeric_id(conn: &Connection, table: &str) -> Result<String, String> {
    for _ in 0..1_000_000 {
        let mut buf = [0u8; 4];
        getrandom::fill(&mut buf).map_err(|error| error.to_string())?;
        let mut id = u32::from_be_bytes(buf) >> 4;
        if id < 100 {
            continue;
        }
        // Keep IDs clear of special playlist ranges.
        if (100_000..=100_099).contains(&id) || (200_000..=200_099).contains(&id) {
            continue;
        }
        let id_str = id.to_string();
        let sql = format!("SELECT 1 FROM {table} WHERE ID = ?1 LIMIT 1");
        let exists: bool = conn
            .prepare(&sql)
            .map_err(|error| error.to_string())?
            .exists(params![id_str.as_str()])
            .map_err(|error| error.to_string())?;
        if !exists {
            return Ok(id_str);
        }
        let _ = &mut id;
    }
    Err(format!("could not generate unused ID for {table}"))
}

pub fn new_uuid() -> String {
    Uuid::new_v4().to_string()
}
