use std::path::{Path, PathBuf};

use sib_core::Pipeline;

use crate::error::ConfigError;
use crate::paths::Paths;

const PIPELINES_DIR: &str = "pipelines";
const EXTENSION: &str = "toml";

#[derive(Debug, Clone)]
pub struct PipelineStore {
    dir: PathBuf,
}

impl PipelineStore {
    pub fn new(paths: &Paths) -> Self {
        Self {
            dir: paths.config_dir.join(PIPELINES_DIR),
        }
    }

    pub fn dir(&self) -> &Path {
        &self.dir
    }

    pub fn path_of(&self, id: &str) -> PathBuf {
        self.dir.join(format!("{id}.{EXTENSION}"))
    }

    pub fn load_all(&self) -> Vec<Pipeline> {
        let Ok(entries) = std::fs::read_dir(&self.dir) else {
            return Vec::new();
        };
        let mut pipelines: Vec<Pipeline> = entries
            .flatten()
            .map(|entry| entry.path())
            .filter(|path| path.extension().is_some_and(|ext| ext == EXTENSION))
            .filter_map(|path| Self::read(&path).ok())
            .collect();
        pipelines.sort_by_key(|p| p.name.to_lowercase());
        pipelines
    }

    pub fn read(path: &Path) -> Result<Pipeline, ConfigError> {
        let raw = std::fs::read_to_string(path).map_err(|source| ConfigError::Read {
            path: path.to_path_buf(),
            source,
        })?;
        toml::from_str(&raw).map_err(|source| ConfigError::Parse {
            path: path.to_path_buf(),
            source,
        })
    }

    pub fn save(&self, pipeline: &Pipeline) -> Result<(), ConfigError> {
        Paths::ensure_dir(&self.dir)?;
        let path = self.path_of(&pipeline.id);
        let raw = toml::to_string_pretty(pipeline)?;
        std::fs::write(&path, raw).map_err(|source| ConfigError::Write { path, source })
    }

    pub fn delete(&self, id: &str) -> Result<(), ConfigError> {
        let path = self.path_of(id);
        match std::fs::remove_file(&path) {
            Ok(()) => Ok(()),
            Err(source) if source.kind() == std::io::ErrorKind::NotFound => Ok(()),
            Err(source) => Err(ConfigError::Write { path, source }),
        }
    }

    pub fn export(&self, pipeline: &Pipeline, to: &Path) -> Result<(), ConfigError> {
        let raw = toml::to_string_pretty(pipeline)?;
        std::fs::write(to, raw).map_err(|source| ConfigError::Write {
            path: to.to_path_buf(),
            source,
        })
    }
}

#[cfg(test)]
mod tests {
    use sib_core::{Step, Variable};

    use super::*;

    fn temp_paths() -> Paths {
        let root = std::env::temp_dir().join(format!(
            "sib-pipelines-{}-{}",
            std::process::id(),
            chrono::Utc::now().timestamp_nanos_opt().unwrap_or_default()
        ));
        Paths {
            config_dir: root.join("config"),
            data_dir: root.join("data"),
            cache_dir: root.join("cache"),
            state_dir: root.join("state"),
        }
    }

    fn sample() -> Pipeline {
        Pipeline {
            id: "brain-evolve".into(),
            name: "Brain evolve".into(),
            description: "night run".into(),
            variables: vec![Variable {
                name: "threads".into(),
                default: "16".into(),
                secret: false,
                description: String::new(),
            }],
            steps: vec![Step {
                name: "build".into(),
                command: "cargo build --release".into(),
                ..Default::default()
            }],
        }
    }

    #[test]
    fn save_load_delete_round_trip() {
        let paths = temp_paths();
        let store = PipelineStore::new(&paths);
        assert!(store.load_all().is_empty());
        store.save(&sample()).expect("save");
        assert_eq!(store.load_all(), vec![sample()]);
        store.delete("brain-evolve").expect("delete");
        assert!(store.load_all().is_empty());
        store.delete("brain-evolve").expect("delete twice is fine");
        let _ = std::fs::remove_dir_all(paths.config_dir.parent().unwrap_or(&paths.config_dir));
    }
}
