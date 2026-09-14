use asiba_core::{ActionRecord, ServerId};
use chrono::DateTime;
use rusqlite::{Connection, Row, params};

use crate::error::StorageError;

struct RawRow {
    at: i64,
    server: String,
    module: String,
    kind: String,
    target: String,
    argument: Option<String>,
    is_success: bool,
    message: String,
}

impl RawRow {
    fn read(row: &Row<'_>) -> rusqlite::Result<Self> {
        Ok(Self {
            at: row.get(0)?,
            server: row.get(1)?,
            module: row.get(2)?,
            kind: row.get(3)?,
            target: row.get(4)?,
            argument: row.get(5)?,
            is_success: row.get(6)?,
            message: row.get(7)?,
        })
    }

    fn into_record(self) -> Option<ActionRecord> {
        Some(ActionRecord {
            at: DateTime::from_timestamp(self.at, 0)?,
            server: ServerId::parse(&self.server).ok()?,
            module: self.module,
            kind: self.kind,
            target: self.target,
            argument: self.argument,
            is_success: self.is_success,
            message: self.message,
        })
    }
}

pub fn insert(connection: &Connection, record: &ActionRecord) -> Result<(), StorageError> {
    connection.execute(
        "INSERT INTO actions (at, server, module, kind, target, argument, ok, message) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
        params![
            record.at.timestamp(),
            record.server.as_str(),
            record.module,
            record.kind,
            record.target,
            record.argument,
            record.is_success,
            record.message,
        ],
    )?;
    Ok(())
}

pub fn list_recent(
    connection: &Connection,
    limit: usize,
) -> Result<Vec<ActionRecord>, StorageError> {
    let mut statement = connection.prepare_cached(
        "SELECT at, server, module, kind, target, argument, ok, message FROM actions ORDER BY id DESC LIMIT ?1",
    )?;
    let rows = statement.query_map(params![limit as i64], RawRow::read)?;
    let mut records = Vec::new();
    for row in rows {
        if let Some(record) = row?.into_record() {
            records.push(record);
        }
    }
    Ok(records)
}
