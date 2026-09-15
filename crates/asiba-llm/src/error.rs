use thiserror::Error;

#[derive(Debug, Error)]
pub enum LlmError {
    #[error("model file not found: {0}")]
    ModelNotFound(String),
    #[error("llama-server binary not found: {0}")]
    BinaryNotFound(String),
    #[error("failed to start llama-server: {0}")]
    Spawn(#[from] std::io::Error),
    #[error("llama-server did not become ready in {0} s")]
    NotReady(u64),
    #[error("request failed: {0}")]
    Request(String),
    #[error("unexpected response: {0}")]
    Response(String),
    #[error("download failed: {0}")]
    Download(String),
    #[error("archive extraction failed: {0}")]
    Extract(String),
}
