use rusqlite::Connection;

use crate::error::StorageError;

const SCHEMA: &str = "
CREATE TABLE IF NOT EXISTS samples (
    server TEXT NOT NULL,
    key TEXT NOT NULL,
    at INTEGER NOT NULL,
    value REAL NOT NULL
);
CREATE INDEX IF NOT EXISTS samples_lookup ON samples(server, key, at);
CREATE TABLE IF NOT EXISTS samples_1m (
    server TEXT NOT NULL,
    key TEXT NOT NULL,
    at INTEGER NOT NULL,
    avg REAL NOT NULL,
    min REAL NOT NULL,
    max REAL NOT NULL,
    PRIMARY KEY (server, key, at)
);
CREATE TABLE IF NOT EXISTS actions (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    at INTEGER NOT NULL,
    server TEXT NOT NULL,
    module TEXT NOT NULL,
    kind TEXT NOT NULL,
    target TEXT NOT NULL,
    argument TEXT,
    ok INTEGER NOT NULL,
    message TEXT NOT NULL
);
CREATE TABLE IF NOT EXISTS pipeline_runs (
    run_id INTEGER PRIMARY KEY,
    server TEXT NOT NULL,
    pipeline TEXT NOT NULL,
    name TEXT NOT NULL,
    started INTEGER NOT NULL,
    finished INTEGER,
    status TEXT NOT NULL,
    steps TEXT NOT NULL
);
CREATE INDEX IF NOT EXISTS pipeline_runs_server ON pipeline_runs(server, started);
CREATE TABLE IF NOT EXISTS samples_1h (
    server TEXT NOT NULL,
    key TEXT NOT NULL,
    at INTEGER NOT NULL,
    avg REAL NOT NULL,
    min REAL NOT NULL,
    max REAL NOT NULL,
    PRIMARY KEY (server, key, at)
);
";

pub fn apply(connection: &Connection) -> Result<(), StorageError> {
    connection.execute_batch("PRAGMA journal_mode = WAL; PRAGMA synchronous = NORMAL;")?;
    connection.execute_batch(SCHEMA)?;
    Ok(())
}
