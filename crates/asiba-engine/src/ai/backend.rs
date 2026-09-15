use std::sync::Arc;

use asiba_ai::{AiError, Anthropic, Backend, Gemini, OpenAi, ProviderConfig};
use asiba_config::{AiConfig, AiProvider};

pub fn build(config: &AiConfig, model: &str) -> Result<Arc<dyn Backend>, AiError> {
    let provider = ProviderConfig {
        api_key: config.api_key.clone(),
        model: model.to_owned(),
        base_url: config.base_url.clone(),
    };
    let backend: Arc<dyn Backend> = match config.provider {
        AiProvider::Gemini => Arc::new(Gemini::new(provider)?),
        AiProvider::OpenAi | AiProvider::OpenAiCompatible => Arc::new(OpenAi::new(provider)?),
        AiProvider::Anthropic => Arc::new(Anthropic::new(provider)?),
    };
    Ok(backend)
}
