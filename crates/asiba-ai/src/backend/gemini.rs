use std::time::Duration;

use serde::{Deserialize, Serialize};
use ureq::Agent;

use super::{Backend, Completion};
use crate::error::AiError;

pub const DEFAULT_MODEL: &str = "gemini-2.5-flash";
const ENDPOINT: &str = "https://generativelanguage.googleapis.com/v1beta/models";
const KEY_HEADER: &str = "x-goog-api-key";
const REQUEST_TIMEOUT: Duration = Duration::from_secs(180);
const TEMPERATURE: f64 = 0.2;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GeminiConfig {
    pub api_key: String,
    pub model: String,
}

pub struct Gemini {
    config: GeminiConfig,
    agent: Agent,
}

#[derive(Serialize)]
struct TextPart<'a> {
    text: &'a str,
}

#[derive(Serialize)]
struct Content<'a> {
    #[serde(skip_serializing_if = "Option::is_none")]
    role: Option<&'a str>,
    parts: Vec<TextPart<'a>>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct GenerationConfig {
    temperature: f64,
    max_output_tokens: usize,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct GenerateRequest<'a> {
    system_instruction: Content<'a>,
    contents: Vec<Content<'a>>,
    generation_config: GenerationConfig,
}

#[derive(Deserialize)]
struct GenerateResponse {
    #[serde(default)]
    candidates: Vec<Candidate>,
    #[serde(default)]
    error: Option<ApiError>,
}

#[derive(Deserialize)]
struct Candidate {
    content: Option<CandidateContent>,
    #[serde(rename = "finishReason")]
    finish_reason: Option<String>,
}

#[derive(Deserialize)]
struct CandidateContent {
    #[serde(default)]
    parts: Vec<ResponsePart>,
}

#[derive(Deserialize)]
struct ResponsePart {
    #[serde(default)]
    text: String,
}

#[derive(Deserialize)]
struct ApiError {
    message: String,
}

impl Gemini {
    pub fn new(config: GeminiConfig) -> Result<Self, AiError> {
        if config.api_key.trim().is_empty() {
            return Err(AiError::MissingKey);
        }
        let agent: Agent = Agent::config_builder()
            .timeout_global(Some(REQUEST_TIMEOUT))
            .http_status_as_error(false)
            .build()
            .into();
        Ok(Self { config, agent })
    }

    fn url(&self) -> String {
        format!("{ENDPOINT}/{}:generateContent", self.config.model)
    }
}

impl Backend for Gemini {
    fn model_name(&self) -> String {
        self.config.model.clone()
    }

    fn complete(&self, request: &Completion) -> Result<String, AiError> {
        let body = GenerateRequest {
            system_instruction: Content {
                role: None,
                parts: vec![TextPart {
                    text: &request.system,
                }],
            },
            contents: vec![Content {
                role: Some("user"),
                parts: vec![TextPart {
                    text: &request.user,
                }],
            }],
            generation_config: GenerationConfig {
                temperature: TEMPERATURE,
                max_output_tokens: request.max_tokens,
            },
        };
        let mut response = self
            .agent
            .post(&self.url())
            .header(KEY_HEADER, &self.config.api_key)
            .send_json(&body)
            .map_err(|e| AiError::Request(e.to_string()))?;
        let status = response.status();
        let parsed: GenerateResponse = response
            .body_mut()
            .read_json()
            .map_err(|e| AiError::Response(format!("HTTP {status}: {e}")))?;
        if let Some(error) = parsed.error {
            return Err(AiError::Api(format!("HTTP {status}: {}", error.message)));
        }
        extract_text(parsed)
    }
}

fn extract_text(parsed: GenerateResponse) -> Result<String, AiError> {
    let candidate = parsed
        .candidates
        .into_iter()
        .next()
        .ok_or_else(|| AiError::Response("no candidates".to_owned()))?;
    let text: String = candidate
        .content
        .map(|c| {
            c.parts
                .into_iter()
                .map(|p| p.text)
                .collect::<Vec<_>>()
                .join("")
        })
        .unwrap_or_default();
    if text.trim().is_empty() {
        let reason = candidate
            .finish_reason
            .unwrap_or_else(|| "empty".to_owned());
        return Err(AiError::Response(format!("empty answer ({reason})")));
    }
    Ok(text.trim().to_owned())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_without_key_fails() {
        let config = GeminiConfig {
            api_key: " ".into(),
            model: DEFAULT_MODEL.into(),
        };
        assert!(matches!(Gemini::new(config), Err(AiError::MissingKey)));
    }

    #[test]
    fn extract_text_joins_parts_and_rejects_empty() {
        let parsed: GenerateResponse = serde_json::from_str(
            r#"{"candidates":[{"content":{"parts":[{"text":"a"},{"text":"b"}]},"finishReason":"STOP"}]}"#,
        )
        .expect("json");
        assert_eq!(extract_text(parsed).expect("text"), "ab");
        let empty: GenerateResponse =
            serde_json::from_str(r#"{"candidates":[{"finishReason":"SAFETY"}]}"#).expect("json");
        assert!(extract_text(empty).is_err());
    }
}
