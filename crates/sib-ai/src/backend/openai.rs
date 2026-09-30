use serde::{Deserialize, Serialize};
use ureq::Agent;

use super::{Backend, Completion, OPENAI_DEFAULT_BASE_URL, ProviderConfig, agent, require_key};
use crate::error::AiError;

pub struct OpenAi {
    config: ProviderConfig,
    agent: Agent,
}

#[derive(Serialize)]
struct Message<'a> {
    role: &'a str,
    content: &'a str,
}

#[derive(Serialize)]
struct ChatRequest<'a> {
    model: &'a str,
    messages: Vec<Message<'a>>,
    max_completion_tokens: usize,
}

#[derive(Deserialize)]
struct ChatResponse {
    #[serde(default)]
    choices: Vec<Choice>,
    #[serde(default)]
    error: Option<ApiError>,
}

#[derive(Deserialize)]
struct Choice {
    message: ChoiceMessage,
}

#[derive(Deserialize)]
struct ChoiceMessage {
    #[serde(default)]
    content: Option<String>,
}

#[derive(Deserialize)]
struct ApiError {
    message: String,
}

impl OpenAi {
    pub fn new(config: ProviderConfig) -> Result<Self, AiError> {
        require_key(&config)?;
        Ok(Self {
            config,
            agent: agent(),
        })
    }

    fn url(&self) -> String {
        let base = self.config.base_url.trim().trim_end_matches('/');
        let base = if base.is_empty() {
            OPENAI_DEFAULT_BASE_URL
        } else {
            base
        };
        format!("{base}/chat/completions")
    }
}

impl Backend for OpenAi {
    fn model_name(&self) -> String {
        self.config.model.clone()
    }

    fn complete(&self, request: &Completion) -> Result<String, AiError> {
        let body = ChatRequest {
            model: &self.config.model,
            messages: vec![
                Message {
                    role: "system",
                    content: &request.system,
                },
                Message {
                    role: "user",
                    content: &request.user,
                },
            ],
            max_completion_tokens: request.max_tokens,
        };
        let mut response = self
            .agent
            .post(&self.url())
            .header("Authorization", &format!("Bearer {}", self.config.api_key))
            .send_json(&body)
            .map_err(|e| AiError::Request(e.to_string()))?;
        let status = response.status();
        let parsed: ChatResponse = response
            .body_mut()
            .read_json()
            .map_err(|e| AiError::Response(format!("HTTP {status}: {e}")))?;
        if let Some(error) = parsed.error {
            return Err(AiError::api(status.as_u16(), error.message));
        }
        parsed
            .choices
            .into_iter()
            .next()
            .and_then(|c| c.message.content)
            .map(|t| t.trim().to_owned())
            .filter(|t| !t.is_empty())
            .ok_or_else(|| AiError::Response("empty answer".to_owned()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn config(base_url: &str) -> ProviderConfig {
        ProviderConfig {
            api_key: "k".into(),
            model: "m".into(),
            base_url: base_url.into(),
        }
    }

    #[test]
    fn url_uses_default_base_when_empty_and_trims_slash() {
        let default = OpenAi::new(config("")).expect("backend");
        assert_eq!(default.url(), "https://api.openai.com/v1/chat/completions");
        let custom = OpenAi::new(config("https://api.groq.com/openai/v1/")).expect("backend");
        assert_eq!(
            custom.url(),
            "https://api.groq.com/openai/v1/chat/completions"
        );
    }
}
