mod anthropic;
mod gemini;
mod openai;

use std::time::Duration;

use ureq::Agent;

pub use anthropic::Anthropic;
pub use gemini::Gemini;
pub use openai::OpenAi;

use crate::error::AiError;

pub const GEMINI_DEFAULT_MODEL: &str = "gemini-2.5-flash";
pub const OPENAI_DEFAULT_MODEL: &str = "gpt-5-mini";
pub const ANTHROPIC_DEFAULT_MODEL: &str = "claude-sonnet-5";
pub const OPENAI_DEFAULT_BASE_URL: &str = "https://api.openai.com/v1";
const REQUEST_TIMEOUT: Duration = Duration::from_secs(180);

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Completion {
    pub system: String,
    pub user: String,
    pub max_tokens: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProviderConfig {
    pub api_key: String,
    pub model: String,
    pub base_url: String,
}

pub trait Backend: Send + Sync {
    fn model_name(&self) -> String;
    fn complete(&self, request: &Completion) -> Result<String, AiError>;
}

pub fn agent() -> Agent {
    Agent::config_builder()
        .timeout_global(Some(REQUEST_TIMEOUT))
        .http_status_as_error(false)
        .build()
        .into()
}

pub fn require_key(config: &ProviderConfig) -> Result<(), AiError> {
    if config.api_key.trim().is_empty() {
        return Err(AiError::MissingKey);
    }
    Ok(())
}
