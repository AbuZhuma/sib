use asiba_core::Severity;
use serde::{Deserialize, Serialize};

pub const DEFAULT_MODEL: &str = "gemini-2.5-flash";
pub const DEFAULT_CONTEXT_TOKENS: usize = 60_000;
pub const DEFAULT_COOLDOWN_SECS: u64 = 1800;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct AiConfig {
    pub consent: bool,
    pub api_key: String,
    pub model: String,
    pub context_tokens: usize,
    pub auto_audit: bool,
    pub auto_audit_min_severity: Severity,
    pub cooldown_secs: u64,
}

impl Default for AiConfig {
    fn default() -> Self {
        Self {
            consent: false,
            api_key: String::new(),
            model: DEFAULT_MODEL.to_owned(),
            context_tokens: DEFAULT_CONTEXT_TOKENS,
            auto_audit: true,
            auto_audit_min_severity: Severity::Warning,
            cooldown_secs: DEFAULT_COOLDOWN_SECS,
        }
    }
}

impl AiConfig {
    pub fn is_ready(&self) -> bool {
        self.consent && !self.api_key.trim().is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ai_config_parses_partial_table_with_defaults() {
        let parsed: AiConfig = toml::from_str("consent = true\napi_key = \"k\"").expect("parse");
        assert!(parsed.is_ready());
        assert_eq!(parsed.model, DEFAULT_MODEL);
        assert_eq!(parsed.auto_audit_min_severity, Severity::Warning);
    }

    #[test]
    fn ai_config_without_consent_is_not_ready() {
        let parsed: AiConfig = toml::from_str("api_key = \"k\"").expect("parse");
        assert!(!parsed.is_ready());
    }
}
