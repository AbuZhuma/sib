mod llama_server;

pub use llama_server::{LlamaConfig, LlamaServer};

use crate::error::LlmError;

pub const LOCALHOST: &str = "127.0.0.1";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Completion {
    pub system: String,
    pub user: String,
    pub max_tokens: usize,
}

pub trait Backend: Send + Sync {
    fn model_name(&self) -> String;
    fn complete(&self, request: &Completion) -> Result<String, LlmError>;
}
