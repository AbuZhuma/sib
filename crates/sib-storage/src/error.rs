#[derive(Debug, thiserror::Error)]
pub enum StorageError {
    #[error("sqlite: {0}")]
    Sqlite(#[from] rusqlite::Error),
    #[error("the writer thread has stopped")]
    WriterStopped,
    #[error("serialization: {0}")]
    Json(#[from] serde_json::Error),
}
