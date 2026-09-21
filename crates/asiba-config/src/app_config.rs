use std::path::{Path, PathBuf};

use asiba_core::{AlertRule, Intervals, Retention};
use serde::{Deserialize, Serialize};

use crate::ai_config::AiConfig;
use crate::error::ConfigError;
use crate::paths::Paths;
use crate::secrets::{AI_KEY_ACCOUNT, SecretStore};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ThemeChoice {
    #[default]
    Dark,
    Light,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct AppConfig {
    pub servers_dir: Option<PathBuf>,
    pub theme: ThemeChoice,
    pub desktop_notifications: bool,
    pub alert_rules: Vec<AlertRule>,
    pub intervals: Intervals,
    pub retention: Retention,
    pub ai: AiConfig,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            servers_dir: None,
            theme: ThemeChoice::default(),
            desktop_notifications: true,
            alert_rules: Vec::new(),
            intervals: Intervals::default(),
            retention: Retention::default(),
            ai: AiConfig::default(),
        }
    }
}

impl AppConfig {
    pub fn load(paths: &Paths) -> Result<Self, ConfigError> {
        let path = paths.config_file();
        if !path.exists() {
            return Ok(Self::default());
        }
        let raw = std::fs::read_to_string(&path).map_err(|source| ConfigError::Read {
            path: path.clone(),
            source,
        })?;
        let config: Self =
            toml::from_str(&raw).map_err(|source| ConfigError::Parse { path, source })?;
        Ok(Self {
            ai: config.ai.with_supported_model(),
            ..config
        })
    }

    pub fn save(&self, paths: &Paths) -> Result<(), ConfigError> {
        Paths::ensure_dir(&paths.config_dir)?;
        let path = paths.config_file();
        let raw = toml::to_string_pretty(self)?;
        std::fs::write(&path, raw).map_err(|source| ConfigError::Write { path, source })
    }

    pub fn load_ai_key(
        &mut self,
        store: &dyn SecretStore,
        paths: &Paths,
    ) -> Result<(), ConfigError> {
        let from_file = std::mem::take(&mut self.ai.api_key);
        if !from_file.trim().is_empty() {
            store.set_named(AI_KEY_ACCOUNT, from_file.trim())?;
            self.save(paths)?;
        }
        self.ai.api_key = store.get_named(AI_KEY_ACCOUNT)?.unwrap_or_default();
        Ok(())
    }

    pub fn servers_dir(&self, paths: &Paths) -> PathBuf {
        self.servers_dir
            .clone()
            .unwrap_or_else(|| paths.default_servers_dir())
    }

    pub fn with_servers_dir(mut self, dir: Option<&Path>) -> Self {
        self.servers_dir = dir.map(Path::to_path_buf);
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_config_roundtrips_through_toml() {
        let config = AppConfig::default();
        let raw = toml::to_string(&config).unwrap_or_default();
        let parsed: AppConfig = toml::from_str(&raw).unwrap_or_default();
        assert_eq!(parsed, config);
    }

    #[test]
    fn alert_rules_roundtrip_through_toml() {
        let config = AppConfig {
            alert_rules: vec![asiba_core::AlertRule {
                id: "load".to_owned(),
                name: "Load".to_owned(),
                metric: "system.load1".to_owned(),
                condition: asiba_core::Condition::Above,
                threshold: 8.0,
                for_secs: 120,
                severity: asiba_core::Severity::Warning,
                builtin: false,
            }],
            ..AppConfig::default()
        };
        let raw = toml::to_string(&config).unwrap_or_default();
        let parsed: AppConfig = toml::from_str(&raw).unwrap_or_default();
        assert_eq!(parsed, config);
    }

    #[test]
    fn api_key_is_never_serialized() {
        let config = AppConfig {
            ai: AiConfig {
                api_key: "secret".to_owned(),
                ..AiConfig::default()
            },
            ..AppConfig::default()
        };
        let raw = toml::to_string(&config).unwrap_or_default();
        assert!(!raw.contains("secret"));
    }

    #[test]
    fn empty_file_gives_defaults() {
        let parsed: AppConfig = toml::from_str("").unwrap_or_default();
        assert_eq!(parsed, AppConfig::default());
    }
}
