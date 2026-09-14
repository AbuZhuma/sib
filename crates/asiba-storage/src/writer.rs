use std::sync::mpsc::{self, RecvTimeoutError, Sender};
use std::time::{Duration, Instant};

use chrono::Utc;

use crate::database::Database;
use crate::error::StorageError;
use crate::sample::StoredSample;

const FLUSH_INTERVAL: Duration = Duration::from_secs(5);
const MAINTENANCE_INTERVAL: Duration = Duration::from_secs(3600);
const MAX_BATCH: usize = 5000;

#[derive(Clone)]
pub struct StorageWriter {
    sender: Sender<StoredSample>,
}

impl StorageWriter {
    pub fn write(&self, sample: StoredSample) -> Result<(), StorageError> {
        self.sender
            .send(sample)
            .map_err(|_| StorageError::WriterStopped)
    }
}

pub fn spawn_writer(mut database: Database) -> StorageWriter {
    let (sender, receiver) = mpsc::channel::<StoredSample>();
    std::thread::Builder::new()
        .name("asiba-storage".to_owned())
        .spawn(move || {
            let mut batch = Vec::new();
            let mut last_flush = Instant::now();
            let mut last_maintenance = Instant::now();
            loop {
                match receiver.recv_timeout(FLUSH_INTERVAL) {
                    Ok(sample) => batch.push(sample),
                    Err(RecvTimeoutError::Timeout) => {}
                    Err(RecvTimeoutError::Disconnected) => break,
                }
                let is_due = last_flush.elapsed() >= FLUSH_INTERVAL || batch.len() >= MAX_BATCH;
                if is_due && !batch.is_empty() {
                    flush(&mut database, &mut batch);
                    last_flush = Instant::now();
                }
                if last_maintenance.elapsed() >= MAINTENANCE_INTERVAL {
                    maintain(&mut database);
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

fn maintain(database: &mut Database) {
    if let Err(error) = database.run_maintenance(Utc::now()) {
        tracing::error!(%error, "обслуживание базы не выполнено");
    }
}
