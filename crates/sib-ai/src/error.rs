use thiserror::Error;

const TRANSIENT_STATUSES: [u16; 6] = [408, 429, 500, 502, 503, 504];

#[derive(Debug, Error)]
pub enum AiError {
    #[error("API key is not set")]
    MissingKey,
    #[error("request failed: {0}")]
    Request(String),
    #[error("API error: HTTP {status}: {message}")]
    Api { status: u16, message: String },
    #[error("unexpected response: {0}")]
    Response(String),
}

impl AiError {
    pub fn api(status: u16, message: impl Into<String>) -> Self {
        Self::Api {
            status,
            message: message.into(),
        }
    }

    pub fn is_transient(&self) -> bool {
        match self {
            Self::Api { status, .. } => TRANSIENT_STATUSES.contains(status),
            Self::Request(_) => true,
            Self::MissingKey | Self::Response(_) => false,
        }
    }
}
