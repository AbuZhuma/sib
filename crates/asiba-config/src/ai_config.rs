use asiba_core::Severity;
use serde::{Deserialize, Serialize};

pub const DEFAULT_CONTEXT_TOKENS: usize = 60_000;
pub const DEFAULT_COOLDOWN_SECS: u64 = 1800;
pub const SECTION_KEYS: [&str; 11] = [
    "processes",
    "resources",
    "ports",
    "docker",
    "services",
    "logs",
    "users",
    "security",
    "anomalies",
    "deploy",
    "gpu",
];
const GEMINI_MODELS: [&str; 7] = [
    "gemini-3.6-flash",
    "gemini-3.7-flash",
    "gemini-3.8-flash",
    "gemini-3.5-flash-lite",
    "gemini-3.1-pro-preview",
    "gemini-flash-latest",
    "gemini-pro-latest",
];
const GEMINI_FALLBACKS: [&str; 2] = ["gemini-3.6-flash", "gemini-flash-latest"];
const OPENAI_FALLBACKS: [&str; 1] = ["gpt-5-mini"];
const ANTHROPIC_FALLBACKS: [&str; 1] = ["claude-sonnet-5"];
const OPENAI_MODELS: [&str; 6] = [
    "gpt-5-mini",
    "gpt-5",
    "gpt-5-nano",
    "gpt-4.1",
    "gpt-4.1-mini",
    "gpt-4o-mini",
];
const ANTHROPIC_MODELS: [&str; 4] = [
    "claude-sonnet-5",
    "claude-opus-5",
    "claude-haiku-4-5-20251001",
    "claude-sonnet-4-5",
];

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AiProvider {
    #[default]
    Gemini,
    OpenAi,
    Anthropic,
    OpenAiCompatible,
}

impl AiProvider {
    pub const ALL: [AiProvider; 4] = [
        AiProvider::Gemini,
        AiProvider::OpenAi,
        AiProvider::Anthropic,
        AiProvider::OpenAiCompatible,
    ];

    pub fn label(self) -> &'static str {
        match self {
            Self::Gemini => "Gemini",
            Self::OpenAi => "OpenAI (GPT)",
            Self::Anthropic => "Claude",
            Self::OpenAiCompatible => "OpenAI-совместимый API",
        }
    }

    pub fn models(self) -> &'static [&'static str] {
        match self {
            Self::Gemini => &GEMINI_MODELS,
            Self::OpenAi | Self::OpenAiCompatible => &OPENAI_MODELS,
            Self::Anthropic => &ANTHROPIC_MODELS,
        }
    }

    pub fn default_model(self) -> &'static str {
        self.models()[0]
    }

    pub fn fallback_models(self) -> &'static [&'static str] {
        match self {
            Self::Gemini => &GEMINI_FALLBACKS,
            Self::OpenAi => &OPENAI_FALLBACKS,
            Self::Anthropic => &ANTHROPIC_FALLBACKS,
            Self::OpenAiCompatible => &[],
        }
    }

    pub fn needs_base_url(self) -> bool {
        self == Self::OpenAiCompatible
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct AiConfig {
    pub consent: bool,
    pub provider: AiProvider,
    #[serde(skip_serializing)]
    pub api_key: String,
    pub model: String,
    pub base_url: String,
    pub context_tokens: usize,
    pub auto_audit: bool,
    pub auto_audit_min_severity: Severity,
    pub cooldown_secs: u64,
    pub auto_sections: bool,
    pub sections: Vec<String>,
}

impl Default for AiConfig {
    fn default() -> Self {
        Self {
            consent: false,
            provider: AiProvider::default(),
            api_key: String::new(),
            model: AiProvider::default().default_model().to_owned(),
            base_url: String::new(),
            context_tokens: DEFAULT_CONTEXT_TOKENS,
            auto_audit: true,
            auto_audit_min_severity: Severity::Warning,
            cooldown_secs: DEFAULT_COOLDOWN_SECS,
            auto_sections: true,
            sections: SECTION_KEYS.iter().map(|k| (*k).to_owned()).collect(),
        }
    }
}

impl AiConfig {
    pub fn with_supported_model(mut self) -> Self {
        let is_known = self.provider.models().contains(&self.model.trim());
        if !is_known && !self.provider.needs_base_url() {
            self.model = self.provider.default_model().to_owned();
        }
        self
    }

    pub fn is_section_auto(&self, key: &str) -> bool {
        self.auto_sections && self.sections.iter().any(|s| s == key)
    }

    pub fn is_ready(&self) -> bool {
        self.consent
            && !self.api_key.trim().is_empty()
            && (!self.provider.needs_base_url() || !self.base_url.trim().is_empty())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ai_config_parses_partial_table_with_defaults() {
        let parsed: AiConfig = toml::from_str("consent = true\napi_key = \"k\"").expect("parse");
        assert!(parsed.is_ready());
        assert_eq!(parsed.provider, AiProvider::Gemini);
        assert_eq!(parsed.model, GEMINI_MODELS[0]);
    }

    #[test]
    fn retired_model_is_replaced_by_the_provider_default() {
        let parsed: AiConfig = toml::from_str("model = \"gemini-2.5-flash\"").expect("parse");
        assert_eq!(parsed.with_supported_model().model, GEMINI_MODELS[0]);
    }

    #[test]
    fn custom_model_of_compatible_provider_is_kept() {
        let parsed: AiConfig =
            toml::from_str("provider = \"open_ai_compatible\"\nmodel = \"llama\"").expect("parse");
        assert_eq!(parsed.with_supported_model().model, "llama");
    }

    #[test]
    fn sections_default_to_all_and_respect_master_switch() {
        let parsed: AiConfig = toml::from_str("").expect("parse");
        assert!(parsed.is_section_auto("docker"));
        let manual: AiConfig =
            toml::from_str("auto_sections = false\nsections = [\"docker\"]").expect("parse");
        assert!(!manual.is_section_auto("docker"));
        let partial: AiConfig = toml::from_str("sections = [\"docker\"]").expect("parse");
        assert!(partial.is_section_auto("docker"));
        assert!(!partial.is_section_auto("logs"));
    }

    #[test]
    fn compatible_provider_requires_base_url() {
        let parsed: AiConfig =
            toml::from_str("consent = true\napi_key = \"k\"\nprovider = \"open_ai_compatible\"")
                .expect("parse");
        assert!(!parsed.is_ready());
    }
}
