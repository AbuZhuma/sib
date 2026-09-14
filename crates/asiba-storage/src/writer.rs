use std::sync::mpsc::{self, RecvTimeoutError, Sender};
use std::time::{Duration, Instant};

use chrono::Utc;

use asiba_core::ActionRecord;

use crate::database::Database;
use crate::error::StorageError;
use crate::maintenance::Retention;
use crate::sample::StoredSample;

const FLUSH_INTERVAL: Duration = Duration::from_secs(5);
const MAINTENANCE_INTERVAL: Duration = Duration::from_secs(3600);
const MAX_BATCH: usize = 5000;

enum WriteRequest {
    Sample(StoredSample),
    Action(ActionRecord),
}

#[derive(Clone)]
pub struct StorageWriter {
    sender: Sender<WriteRequest>,
}

impl StorageWriter {
    pub fn write(&self, sample: StoredSample) -> Result<(), StorageError> {
        self.sender
            .send(WriteRequest::Sample(sample))
            .map_err(|_| StorageError::WriterStopped)
    }

    pub fn write_action(&self, record: ActionRecord) -> Result<(), StorageError> {
        self.sender
            .send(WriteRequest::Action(record))
            .map_err(|_| StorageError::WriterStopped)
    }
}

pub fn spawn_writer(mut database: Database, retention: Retention) -> StorageWriter {
    let (sender, receiver) = mpsc::channel::<WriteRequest>();
    std::thread::Builder::new()
        .name("asiba-storage".to_owned())
        .spawn(move || {
            let mut batch = Vec::new();
            let mut last_flush = Instant::now();
            let mut last_maintenance = Instant::now();
            loop {
                match receiver.recv_timeout(FLUSH_INTERVAL) {
                    Ok(WriteRequest::Sample(sample)) => batch.push(sample),
                    Ok(WriteRequest::Action(record)) => record_action(&database, &record),
                    Err(RecvTimeoutError::Timeout) => {}
                    Err(RecvTimeoutError::Disconnected) => break,
                }
                let is_due = last_flush.elapsed() >= FLUSH_INTERVAL || batch.len() >= MAX_BATCH;
                if is_due && !batch.is_empty() {
                    flush(&mut database, &mut batch);
                    last_flush = Instant::now();
                }
                if last_maintenance.elapsed() >= MAINTENANCE_INTERVAL {
                    maintain(&mut database, &retention);
                    last_maintenance = Instant::now();
                }
            }
            flush(&mut database, &mut batch);
        })
        .ok();
    StorageWriter { sender }
}

fn flush(database: &mut Database, batch: &mut Vec<StoredSample>) {
    if let Err(error) = database.insert_batch(batch) {
        tracing::error!(%error, "не удалось записать метрики");
    }
    batch.clear();
}

fn record_action(database: &Database, record: &ActionRecord) {
    if let Err(error) = database.insert_action(record) {
        tracing::error!(%error, "не удалось записать действие в журнал");
    }
}

fn maintain(database: &mut Database, retention: &Retention) {
    if let Err(error) = database.run_maintenance(Utc::now(), retention) {
        tracing::error!(%error, "обслуживание базы не выполнено");
    }
}
