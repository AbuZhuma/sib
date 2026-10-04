use std::path::Path;

use chrono::{DateTime, Utc};
use rusqlite::{Connection, params};
use sib_core::Point;

use sib_core::{ActionRecord, PipelineRun};

use crate::actions;
use crate::error::StorageError;
use crate::pipeline_runs;
use sib_core::Retention;

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

    pub fn insert_action(&self, record: &ActionRecord) -> Result<(), StorageError> {
        actions::insert(&self.connection, record)
    }

    pub fn recent_actions(&self, limit: usize) -> Result<Vec<ActionRecord>, StorageError> {
        actions::list_recent(&self.connection, limit)
    }

    pub fn upsert_pipeline_run(&self, run: &PipelineRun) -> Result<(), StorageError> {
        pipeline_runs::upsert(&self.connection, run)
    }

    pub fn recent_pipeline_runs(&self, limit: usize) -> Result<Vec<PipelineRun>, StorageError> {
        pipeline_runs::list_recent(&self.connection, limit)
    }

    pub fn run_maintenance(
        &mut self,
        now: DateTime<Utc>,
        retention: &Retention,
    ) -> Result<(), StorageError> {
        maintenance::run(&mut self.connection, now, retention)
    }

    pub fn delete_server(&self, server: &str) -> Result<(), StorageError> {
        for table in [
            "samples",
            "samples_1m",
            "samples_1h",
            "actions",
            "pipeline_runs",
        ] {
            let query = format!("DELETE FROM {table} WHERE server = ?1");
            self.connection.execute(&query, params![server])?;
        }
        Ok(())
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
    fn pipeline_run_round_trips() {
        let db = Database::in_memory().expect("db");
        let run = PipelineRun {
            id: 42,
            server: sib_core::ServerId::parse("neo").expect("id"),
            pipeline: "deploy".into(),
            pipeline_name: "Deploy".into(),
            started_at: DateTime::from_timestamp(1_700_000_000, 0).expect("time"),
            finished_at: Some(DateTime::from_timestamp(1_700_000_100, 0).expect("time")),
            status: sib_core::RunStatus::Failed("step build exited with 1".into()),
            steps: vec![sib_core::StepRun {
                name: "build".into(),
                target: sib_core::StepTarget::Server,
                status: sib_core::StepStatus::Failed,
                exit_code: Some(1),
                output: "error\n".into(),
                started_at: None,
                finished_at: None,
            }],
        };
        db.upsert_pipeline_run(&run).expect("insert");
        db.upsert_pipeline_run(&run).expect("replace");
        assert_eq!(db.recent_pipeline_runs(10).expect("list"), vec![run]);
        db.delete_server("neo").expect("delete");
        assert!(db.recent_pipeline_runs(10).expect("list").is_empty());
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
    fn insert_action_then_recent_returns_newest_first() {
        let db = Database::in_memory().expect("db");
        let record = |kind: &str| ActionRecord {
            at: Utc::now(),
            server: sib_core::ServerId::parse("neo").expect("id"),
            module: "security".to_owned(),
            kind: kind.to_owned(),
            target: "1.2.3.4".to_owned(),
            argument: None,
            is_success: true,
            message: "ok".to_owned(),
        };
        db.insert_action(&record("ban")).expect("insert");
        db.insert_action(&record("unban")).expect("insert");
        let recent = db.recent_actions(10).expect("recent");
        let kinds: Vec<&str> = recent.iter().map(|r| r.kind.as_str()).collect();
        assert_eq!(kinds, vec!["unban", "ban"]);
        assert_eq!(recent[0].server.as_str(), "neo");
    }

    #[test]
    fn delete_server_removes_its_rows_only() {
        let mut db = Database::in_memory().expect("db");
        let now = Utc::now();
        let mut other = sample(now, 2.0);
        other.server = "other".into();
        db.insert_batch(&[sample(now, 1.0), other]).expect("insert");
        db.delete_server("neo").expect("delete");
        assert_eq!(db.count("samples").expect("count"), 1);
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
