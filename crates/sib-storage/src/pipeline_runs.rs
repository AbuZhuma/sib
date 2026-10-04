use chrono::DateTime;
use rusqlite::{Connection, Row, params};
use sib_core::{PipelineRun, RunStatus, ServerId, StepRun};

use crate::error::StorageError;

struct RawRow {
    run_id: i64,
    server: String,
    pipeline: String,
    name: String,
    started: i64,
    finished: Option<i64>,
    status: String,
    steps: String,
}

impl RawRow {
    fn read(row: &Row<'_>) -> rusqlite::Result<Self> {
        Ok(Self {
            run_id: row.get(0)?,
            server: row.get(1)?,
            pipeline: row.get(2)?,
            name: row.get(3)?,
            started: row.get(4)?,
            finished: row.get(5)?,
            status: row.get(6)?,
            steps: row.get(7)?,
        })
    }

    fn into_run(self) -> Option<PipelineRun> {
        let steps: Vec<StepRun> = serde_json::from_str(&self.steps).ok()?;
        let status: RunStatus = serde_json::from_str(&self.status).ok()?;
        Some(PipelineRun {
            id: self.run_id as u64,
            server: ServerId::parse(&self.server).ok()?,
            pipeline: self.pipeline,
            pipeline_name: self.name,
            started_at: DateTime::from_timestamp(self.started, 0)?,
            finished_at: self.finished.and_then(|t| DateTime::from_timestamp(t, 0)),
            status,
            steps,
        })
    }
}

pub fn upsert(connection: &Connection, run: &PipelineRun) -> Result<(), StorageError> {
    let steps = serde_json::to_string(&run.steps)?;
    let status = serde_json::to_string(&run.status)?;
    connection.execute(
        "INSERT OR REPLACE INTO pipeline_runs (run_id, server, pipeline, name, started, finished, status, steps) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
        params![
            run.id as i64,
            run.server.as_str(),
            run.pipeline,
            run.pipeline_name,
            run.started_at.timestamp(),
            run.finished_at.map(|t| t.timestamp()),
            status,
            steps,
        ],
    )?;
    Ok(())
}

pub fn list_recent(
    connection: &Connection,
    limit: usize,
) -> Result<Vec<PipelineRun>, StorageError> {
    let mut statement = connection.prepare_cached(
        "SELECT run_id, server, pipeline, name, started, finished, status, steps FROM pipeline_runs ORDER BY started DESC LIMIT ?1",
    )?;
    let rows = statement.query_map(params![limit as i64], RawRow::read)?;
    let mut runs = Vec::new();
    for row in rows {
        if let Some(run) = row?.into_run() {
            runs.push(run);
        }
    }
    runs.reverse();
    Ok(runs)
}
