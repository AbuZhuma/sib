use std::path::{Path, PathBuf};

use directories::ProjectDirs;

use crate::error::ConfigError;

const APP_NAME: &str = "asiba";
const SERVERS_DIR: &str = "servers";
const CONFIG_FILE: &str = "config.toml";
const HISTORY_DB: &str = "history.db";

#[derive(Debug, Clone)]
pub struct Paths {
    pub config_dir: PathBuf,
    pub data_dir: PathBuf,
    pub cache_dir: PathBuf,
    pub state_dir: PathBuf,
}

impl Paths {
    pub fn discover() -> Result<Self, ConfigError> {
        let dirs = ProjectDirs::from("", "", APP_NAME).ok_or(ConfigError::NoHome)?;
        let state_dir = dirs
            .state_dir()
            .unwrap_or_else(|| dirs.data_local_dir())
            .to_path_buf();
        Ok(Self {
            config_dir: dirs.config_dir().to_path_buf(),
            data_dir: dirs.data_dir().to_path_buf(),
            cache_dir: dirs.cache_dir().to_path_buf(),
            state_dir,
        })
    }

    pub fn config_file(&self) -> PathBuf {
        self.config_dir.join(CONFIG_FILE)
    }

    pub fn history_db(&self) -> PathBuf {
        self.data_dir.join(HISTORY_DB)
    }

    pub fn default_servers_dir(&self) -> PathBuf {
        self.config_dir.join(SERVERS_DIR)
    }

    pub fn ensure_dir(dir: &Path) -> Result<(), ConfigError> {
        std::fs::create_dir_all(dir).map_err(|source| ConfigError::Write {
            path: dir.to_path_buf(),
            source,
        })
    }
}
