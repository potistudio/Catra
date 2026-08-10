use rusqlite::{params, Connection, OptionalExtension};

/// Pending row that needs `rb_local_usn` assigned on commit.
#[derive(Debug, Clone)]
pub struct UsnTarget {
    pub table: &'static str,
    pub id: String,
}

#[derive(Debug, Default)]
pub struct UsnBuffer {
    targets: Vec<UsnTarget>,
}

impl UsnBuffer {
    pub fn track(&mut self, table: &'static str, id: impl Into<String>) {
        self.targets.push(UsnTarget {
            table,
            id: id.into(),
        });
    }

    pub fn track_many(&mut self, table: &'static str, ids: impl IntoIterator<Item = String>) {
        for id in ids {
            self.track(table, id);
        }
    }

    /// Increment `agentRegistry.localUpdateCount` and stamp each pending row.
    pub fn apply(self, conn: &Connection) -> Result<i64, String> {
        let mut usn = local_update_count(conn)?;
        for target in self.targets {
            usn += 1;
            let sql = format!(
                "UPDATE {} SET rb_local_usn = ?1, updated_at = COALESCE(updated_at, CURRENT_TIMESTAMP) WHERE ID = ?2",
                target.table
            );
            conn.execute(&sql, params![usn, target.id.as_str()])
                .map_err(|error| format!("failed to set USN on {} {}: {error}", target.table, target.id))?;
        }
        set_local_update_count(conn, usn)?;
        Ok(usn)
    }
}

pub fn local_update_count(conn: &Connection) -> Result<i64, String> {
    conn.query_row(
        "SELECT int_1 FROM agentRegistry WHERE registry_id = 'localUpdateCount'",
        [],
        |row| row.get::<_, Option<i64>>(0),
    )
    .optional()
    .map_err(|error| error.to_string())?
    .flatten()
    .ok_or_else(|| "agentRegistry localUpdateCount is missing".to_string())
}

fn set_local_update_count(conn: &Connection, value: i64) -> Result<(), String> {
    let updated = conn
        .execute(
            "UPDATE agentRegistry SET int_1 = ?1, updated_at = CURRENT_TIMESTAMP WHERE registry_id = 'localUpdateCount'",
            params![value],
        )
        .map_err(|error| error.to_string())?;
    if updated == 0 {
        return Err("agentRegistry localUpdateCount row was not updated".to_string());
    }
    Ok(())
}
