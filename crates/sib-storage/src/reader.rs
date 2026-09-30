use std::collections::BTreeMap;
use std::path::Path;

use chrono::{DateTime, Utc};
use rusqlite::{Connection, OpenFlags, params};
use sib_core::{ActionRecord, Point};

use crate::actions;
use crate::error::StorageError;

pub struct HistoryReader {
    connection: Connection,
}

impl HistoryReader {
    pub fn open(path: &Path) -> Result<Self, StorageError> {
        let flags = OpenFlags::SQLITE_OPEN_READ_ONLY | OpenFlags::SQLITE_OPEN_NO_MUTEX;
        Ok(Self {
            connection: Connection::open_with_flags(path, flags)?,
        })
    }

    pub fn recent_actions(&self, limit: usize) -> Result<Vec<ActionRecord>, StorageError> {
        actions::list_recent(&self.connection, limit)
    }

    pub fn load_series(
        &self,
        server: &str,
        since: DateTime<Utc>,
    ) -> Result<BTreeMap<String, Vec<Point>>, StorageError> {
        let mut statement = self.connection.prepare_cached(
            "SELECT key, at, value FROM samples WHERE server = ?1 AND at >= ?2 ORDER BY key, at",
        )?;
        let rows = statement.query_map(params![server, since.timestamp()], |row| {
            let key: String = row.get(0)?;
            let at: i64 = row.get(1)?;
            let value: f64 = row.get(2)?;
            Ok((key, at, value))
        })?;
        let mut series: BTreeMap<String, Vec<Point>> = BTreeMap::new();
        for row in rows {
            let (key, at, value) = row?;
            let at = DateTime::from_timestamp(at, 0).unwrap_or_default();
            series.entry(key).or_default().push(Point { at, value });
        }
        Ok(series)
    }
}
