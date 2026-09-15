use std::path::PathBuf;

use asiba_core::Severity;
use serde::{Deserialize, Serialize};

pub const DEFAULT_SERVER_BINARY: &str = "llama-server";
pub const DEFAULT_CONTEXT_TOKENS: usize = 8192;
pub const DEFAULT_COOLDOWN_SECS: u64 = 1800;
pub const DEFAULT_IDLE_UNLOAD_SECS: u64 = 600;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct LlmConfig {
    pub enabled: bool,
    pub model_path: Option<PathBuf>,
    pub server_binary: String,
    pub threads: usize,
    pub context_tokens: usize,
    pub auto_audit: bool,
    pub auto_audit_min_severity: Severity,
    pub cooldown_secs: u64,
    pub idle_unload_secs: u64,
}

impl Default for LlmConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            model_path: None,
            server_binary: DEFAULT_SERVER_BINARY.to_owned(),
            threads: 0,
            context_tokens: DEFAULT_CONTEXT_TOKENS,
            auto_audit: true,
            auto_audit_min_severity: Severity::Warning,
            cooldown_secs: DEFAULT_COOLDOWN_SECS,
            idle_unload_secs: DEFAULT_IDLE_UNLOAD_SECS,
        }
    }
}

impl LlmConfig {
    pub fn is_ready(&self) -> bool {
        self.enabled && self.model_path.as_ref().is_some_and(|p| p.is_file())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn llm_config_parses_partial_table_with_defaults() {
        let parsed: LlmConfig =
            toml::from_str("enabled = true\nmodel_path = \"/m.gguf\"").expect("parse");
        assert!(parsed.enabled);
        assert_eq!(parsed.model_path, Some(PathBuf::from("/m.gguf")));
        assert_eq!(parsed.context_tokens, DEFAULT_CONTEXT_TOKENS);
        assert_eq!(parsed.auto_audit_min_severity, Severity::Warning);
    }
}
