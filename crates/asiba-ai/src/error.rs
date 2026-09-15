use thiserror::Error;

#[derive(Debug, Error)]
pub enum AiError {
    #[error("API key is not set")]
    MissingKey,
    #[error("request failed: {0}")]
    Request(String),
    #[error("API error: {0}")]
    Api(String),
    #[error("unexpected response: {0}")]
    Response(String),
}
