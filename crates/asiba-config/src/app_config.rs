use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::error::ConfigError;
use crate::paths::Paths;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ThemeChoice {
    #[default]
    Dark,
    Light,
}

#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct AppConfig {
    pub servers_dir: Option<PathBuf>,
    pub theme: ThemeChoice,
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
        toml::from_str(&raw).map_err(|source| ConfigError::Parse { path, source })
    }

    pub fn save(&self, paths: &Paths) -> Result<(), ConfigError> {
        Paths::ensure_dir(&paths.config_dir)?;
        let path = paths.config_file();
        let raw = toml::to_string_pretty(self)?;
        std::fs::write(&path, raw).map_err(|source| ConfigError::Write { path, source })
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
    fn empty_file_gives_defaults() {
        let parsed: AppConfig = toml::from_str("").unwrap_or_default();
        assert_eq!(parsed, AppConfig::default());
    }
}
