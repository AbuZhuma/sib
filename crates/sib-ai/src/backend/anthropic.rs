use serde::{Deserialize, Serialize};
use ureq::Agent;

use super::{Backend, Completion, ProviderConfig, agent, require_key};
use crate::error::AiError;

const ENDPOINT: &str = "https://api.anthropic.com/v1/messages";
const VERSION: &str = "2023-06-01";

pub struct Anthropic {
    config: ProviderConfig,
    agent: Agent,
}

#[derive(Serialize)]
struct Message<'a> {
    role: &'a str,
    content: &'a str,
}

#[derive(Serialize)]
struct MessagesRequest<'a> {
    model: &'a str,
    max_tokens: usize,
    system: &'a str,
    messages: Vec<Message<'a>>,
}

#[derive(Deserialize)]
struct MessagesResponse {
    #[serde(default)]
    content: Vec<Block>,
    #[serde(default)]
    error: Option<ApiError>,
}

#[derive(Deserialize)]
struct Block {
    #[serde(default)]
    text: String,
}

#[derive(Deserialize)]
struct ApiError {
    message: String,
}

impl Anthropic {
    pub fn new(config: ProviderConfig) -> Result<Self, AiError> {
        require_key(&config)?;
        Ok(Self {
            config,
            agent: agent(),
        })
    }
}

impl Backend for Anthropic {
    fn model_name(&self) -> String {
        self.config.model.clone()
    }

    fn complete(&self, request: &Completion) -> Result<String, AiError> {
        let body = MessagesRequest {
            model: &self.config.model,
            max_tokens: request.max_tokens,
            system: &request.system,
            messages: vec![Message {
                role: "user",
                content: &request.user,
            }],
        };
        let mut response = self
            .agent
            .post(ENDPOINT)
            .header("x-api-key", &self.config.api_key)
            .header("anthropic-version", VERSION)
            .send_json(&body)
            .map_err(|e| AiError::Request(e.to_string()))?;
        let status = response.status();
        let parsed: MessagesResponse = response
            .body_mut()
            .read_json()
            .map_err(|e| AiError::Response(format!("HTTP {status}: {e}")))?;
        if let Some(error) = parsed.error {
            return Err(AiError::api(status.as_u16(), error.message));
        }
        let text: String = parsed.content.into_iter().map(|b| b.text).collect();
        let text = text.trim().to_owned();
        if text.is_empty() {
            return Err(AiError::Response("empty answer".to_owned()));
        }
        Ok(text)
    }
}
