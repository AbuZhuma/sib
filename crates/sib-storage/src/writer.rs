use std::sync::mpsc::{self, RecvTimeoutError, Sender};
use std::time::{Duration, Instant};

use chrono::Utc;
use sib_core::{ActionRecord, Retention};

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

struct Writer {
    database: Database,
    retention: Retention,
    batch: Vec<StoredSample>,
    last_flush: Instant,
    last_maintenance: Instant,
}

impl Writer {
    fn new(mut database: Database, retention: Retention) -> Self {
        maintain(&mut database, &retention);
        Self {
            database,
            retention,
            batch: Vec::new(),
            last_flush: Instant::now(),
            last_maintenance: Instant::now(),
        }
    }

    fn accept(&mut self, request: WriteRequest) {
        match request {
            WriteRequest::Sample(sample) => self.batch.push(sample),
            WriteRequest::Action(record) => record_action(&self.database, &record),
            WriteRequest::DeleteServer(server) => {
                flush(&mut self.database, &mut self.batch);
                delete_server(&self.database, &server);
            }
            WriteRequest::Retention(next) => {
                self.retention = next;
                maintain(&mut self.database, &self.retention);
                self.last_maintenance = Instant::now();
            }
        }
    }

    fn tick(&mut self) {
        let is_due = self.last_flush.elapsed() >= FLUSH_INTERVAL || self.batch.len() >= MAX_BATCH;
        if is_due && !self.batch.is_empty() {
            flush(&mut self.database, &mut self.batch);
            self.last_flush = Instant::now();
        }
        if self.last_maintenance.elapsed() >= MAINTENANCE_INTERVAL {
            maintain(&mut self.database, &self.retention);
            self.last_maintenance = Instant::now();
        }
    }
}

pub fn spawn_writer(database: Database, retention: Retention) -> StorageWriter {
    let (sender, receiver) = mpsc::channel::<WriteRequest>();
    std::thread::Builder::new()
        .name("sib-storage".to_owned())
        .spawn(move || {
            let mut writer = Writer::new(database, retention);
            loop {
                match receiver.recv_timeout(FLUSH_INTERVAL) {
                    Ok(request) => writer.accept(request),
                    Err(RecvTimeoutError::Timeout) => {}
                    Err(RecvTimeoutError::Disconnected) => break,
                }
                writer.tick();
            }
            flush(&mut writer.database, &mut writer.batch);
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
