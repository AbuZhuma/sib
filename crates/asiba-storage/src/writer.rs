use std::sync::mpsc::{self, RecvTimeoutError, Sender};
use std::time::{Duration, Instant};

use asiba_core::{ActionRecord, Retention};
use chrono::Utc;

use crate::database::Database;
use crate::error::StorageError;
use crate::sample::StoredSample;

const FLUSH_INTERVAL: Duration = Duration::from_secs(5);
const MAINTENANCE_INTERVAL: Duration = Duration::from_secs(3600);
const MAX_BATCH: usize = 5000;

enum WriteRequest {
    Sample(StoredSample),
    Action(ActionRecord),
    DeleteServer(String),
    Retention(Retention),
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
        self.send(WriteRequest::Action(record))
    }

    pub fn delete_server(&self, server: &str) -> Result<(), StorageError> {
        self.send(WriteRequest::DeleteServer(server.to_owned()))
    }

    pub fn set_retention(&self, retention: Retention) -> Result<(), StorageError> {
        self.send(WriteRequest::Retention(retention))
    }

    fn send(&self, request: WriteRequest) -> Result<(), StorageError> {
        self.sender
            .send(request)
            .map_err(|_| StorageError::WriterStopped)
    }
}

pub fn spawn_writer(mut database: Database, retention: Retention) -> StorageWriter {
    let (sender, receiver) = mpsc::channel::<WriteRequest>();
    std::thread::Builder::new()
        .name("asiba-storage".to_owned())
        .spawn(move || {
            let mut retention = retention;
            let mut batch = Vec::new();
            let mut last_flush = Instant::now();
            maintain(&mut database, &retention);
            let mut last_maintenance = Instant::now();
            loop {
                match receiver.recv_timeout(FLUSH_INTERVAL) {
                    Ok(WriteRequest::Sample(sample)) => batch.push(sample),
                    Ok(WriteRequest::Action(record)) => record_action(&database, &record),
                    Ok(WriteRequest::DeleteServer(server)) => {
                        flush(&mut database, &mut batch);
                        delete_server(&database, &server);
                    }
                    Ok(WriteRequest::Retention(next)) => {
                        retention = next;
                        maintain(&mut database, &retention);
                        last_maintenance = Instant::now();
                    }
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

fn delete_server(database: &Database, server: &str) {
    if let Err(error) = database.delete_server(server) {
        tracing::error!(%error, server, "история сервера не удалена");
    }
}

fn maintain(database: &mut Database, retention: &Retention) {
    if let Err(error) = database.run_maintenance(Utc::now(), retention) {
        tracing::error!(%error, "обслуживание базы не выполнено");
    }
}
