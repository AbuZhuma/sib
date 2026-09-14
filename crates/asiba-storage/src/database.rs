use std::path::Path;

use asiba_core::Point;
use chrono::{DateTime, Utc};
use rusqlite::{Connection, params};

use crate::error::StorageError;
use crate::maintenance;
use crate::sample::StoredSample;
use crate::schema;

pub struct Database {
    connection: Connection,
}

impl Database {
    pub fn open(path: &Path) -> Result<Self, StorageError> {
        if let Some(parent) = path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        Self::from_connection(Connection::open(path)?)
    }

    pub fn in_memory() -> Result<Self, StorageError> {
        Self::from_connection(Connection::open_in_memory()?)
    }

    fn from_connection(connection: Connection) -> Result<Self, StorageError> {
        schema::apply(&connection)?;
        Ok(Self { connection })
    }

    pub fn insert_batch(&mut self, samples: &[StoredSample]) -> Result<(), StorageError> {
        let transaction = self.connection.transaction()?;
        {
            let mut statement = transaction.prepare_cached(
                "INSERT INTO samples (server, key, at, value) VALUES (?1, ?2, ?3, ?4)",
            )?;
            for sample in samples {
                statement.execute(params![
                    sample.server,
                    sample.key,
                    sample.at.timestamp(),
                    sample.value
                ])?;
            }
        }
        transaction.commit()?;
        Ok(())
    }

    pub fn query_raw(
        &self,
        server: &str,
        key: &str,
        range: (DateTime<Utc>, DateTime<Utc>),
    ) -> Result<Vec<Point>, StorageError> {
        let mut statement = self.connection.prepare_cached(
            "SELECT at, value FROM samples WHERE server = ?1 AND key = ?2 AND at BETWEEN ?3 AND ?4 ORDER BY at",
        )?;
        let rows = statement.query_map(
            params![server, key, range.0.timestamp(), range.1.timestamp()],
            |row| {
                let at: i64 = row.get(0)?;
                let value: f64 = row.get(1)?;
                Ok((at, value))
            },
        )?;
        let mut points = Vec::new();
        for row in rows {
            let (at, value) = row?;
            let at = DateTime::from_timestamp(at, 0).unwrap_or_default();
            points.push(Point { at, value });
        }
        Ok(points)
    }

    pub fn run_maintenance(&mut self, now: DateTime<Utc>) -> Result<(), StorageError> {
        maintenance::run(&mut self.connection, now)
    }

    pub fn count(&self, table: &str) -> Result<i64, StorageError> {
        let query = format!("SELECT COUNT(*) FROM {table}");
        Ok(self.connection.query_row(&query, [], |row| row.get(0))?)
    }
}

#[cfg(test)]
mod tests {
    use chrono::Duration;

    use super::*;

    fn sample(at: DateTime<Utc>, value: f64) -> StoredSample {
        StoredSample {
            server: "neo".into(),
            key: "cpu.total".into(),
            at,
            value,
        }
    }

    #[test]
    fn insert_then_query_returns_points_in_order() {
        let mut db = Database::in_memory().expect("db");
        let now = Utc::now();
        db.insert_batch(&[sample(now, 2.0), sample(now - Duration::seconds(10), 1.0)])
            .expect("insert");
        let points = db
            .query_raw("neo", "cpu.total", (now - Duration::minutes(1), now))
            .expect("query");
        let values: Vec<f64> = points.iter().map(|p| p.value).collect();
        assert_eq!(values, vec![1.0, 2.0]);
    }

    #[test]
    fn query_other_server_is_empty() {
        let mut db = Database::in_memory().expect("db");
        let now = Utc::now();
        db.insert_batch(&[sample(now, 2.0)]).expect("insert");
        let points = db
            .query_raw("other", "cpu.total", (now - Duration::minutes(1), now))
            .expect("query");
        assert!(points.is_empty());
    }
}
