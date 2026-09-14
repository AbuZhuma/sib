use std::path::{Path, PathBuf};

use asiba_core::{ServerId, ServerSpec};

use crate::error::ConfigError;
use crate::paths::Paths;

const SPEC_EXTENSION: &str = "toml";

#[derive(Debug, Clone)]
pub struct ServerStore {
    dir: PathBuf,
}

impl ServerStore {
    pub fn new(dir: PathBuf) -> Self {
        Self { dir }
    }

    pub fn dir(&self) -> &Path {
        &self.dir
    }

    pub fn spec_path(&self, id: &ServerId) -> PathBuf {
        self.dir.join(format!("{id}.{SPEC_EXTENSION}"))
    }

    pub fn load_all(&self) -> Result<Vec<ServerSpec>, ConfigError> {
        if !self.dir.exists() {
            return Ok(Vec::new());
        }
        let entries = std::fs::read_dir(&self.dir).map_err(|source| ConfigError::Read {
            path: self.dir.clone(),
            source,
        })?;
        let mut specs = Vec::new();
        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().and_then(|e| e.to_str()) != Some(SPEC_EXTENSION) {
                continue;
            }
            match load_spec(&path) {
                Ok(spec) => specs.push(spec),
                Err(error) => tracing::warn!(?path, %error, "пропущен файл сервера"),
            }
        }
        specs.sort_by(|a, b| a.id.cmp(&b.id));
        Ok(specs)
    }

    pub fn save(&self, spec: &ServerSpec) -> Result<(), ConfigError> {
        Paths::ensure_dir(&self.dir)?;
        let path = self.spec_path(&spec.id);
        let raw = toml::to_string_pretty(spec)?;
        std::fs::write(&path, raw).map_err(|source| ConfigError::Write { path, source })
    }

    pub fn delete(&self, id: &ServerId) -> Result<(), ConfigError> {
        let path = self.spec_path(id);
        if !path.exists() {
            return Ok(());
        }
        std::fs::remove_file(&path).map_err(|source| ConfigError::Write { path, source })
    }
}

fn load_spec(path: &Path) -> Result<ServerSpec, ConfigError> {
    let raw = std::fs::read_to_string(path).map_err(|source| ConfigError::Read {
        path: path.to_path_buf(),
        source,
    })?;
    toml::from_str(&raw).map_err(|source| ConfigError::Parse {
        path: path.to_path_buf(),
        source,
    })
}

#[cfg(test)]
mod tests {
    use asiba_core::{AuthMethod, ServerDescription, SudoMode};

    use super::*;

    fn sample(name: &str) -> ServerSpec {
        ServerSpec {
            id: ServerId::parse(name).expect("valid name"),
            host: "10.0.0.1".to_owned(),
            port: 22,
            user: "root".to_owned(),
            auth: AuthMethod::Auto,
            jump: None,
            sudo: SudoMode::None,
            description: ServerDescription::default(),
        }
    }

    fn temp_store() -> ServerStore {
        let unique = format!("asiba-test-{}-{}", std::process::id(), rand_suffix());
        ServerStore::new(std::env::temp_dir().join(unique))
    }

    fn rand_suffix() -> u128 {
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or_default()
    }

    #[test]
    fn save_then_load_all_returns_spec() {
        let store = temp_store();
        let spec = sample("neo");
        store.save(&spec).expect("save");
        let loaded = store.load_all().expect("load");
        let _ = std::fs::remove_dir_all(store.dir());
        assert_eq!(loaded, vec![spec]);
    }

    #[test]
    fn delete_removes_file() {
        let store = temp_store();
        let spec = sample("neo");
        store.save(&spec).expect("save");
        store.delete(&spec.id).expect("delete");
        let loaded = store.load_all().expect("load");
        let _ = std::fs::remove_dir_all(store.dir());
        assert!(loaded.is_empty());
    }

    #[test]
    fn load_all_missing_dir_is_empty() {
        let store = temp_store();
        assert!(store.load_all().expect("load").is_empty());
    }
}
