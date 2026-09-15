mod gemini;

pub use gemini::{DEFAULT_MODEL, Gemini, GeminiConfig};

use crate::error::AiError;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Completion {
    pub system: String,
    pub user: String,
    pub max_tokens: usize,
}

pub trait Backend: Send + Sync {
    fn model_name(&self) -> String;
    fn complete(&self, request: &Completion) -> Result<String, AiError>;
}
