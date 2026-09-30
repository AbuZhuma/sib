use std::path::PathBuf;

use serde::{Deserialize, Serialize};
use sib_core::IgnoredIncident;

use crate::error::ConfigError;
use crate::paths::Paths;

const IGNORED_FILE: &str = "ignored.toml";

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct IgnoredStore {
    pub incidents: Vec<IgnoredIncident>,
}

impl IgnoredStore {
    fn path(paths: &Paths) -> PathBuf {
        paths.state_dir.join(IGNORED_FILE)
    }

    pub fn load(paths: &Paths) -> Self {
        std::fs::read_to_string(Self::path(paths))
            .ok()
            .and_then(|raw| toml::from_str(&raw).ok())
            .unwrap_or_default()
    }

    pub fn save(&self, paths: &Paths) -> Result<(), ConfigError> {
        Paths::ensure_dir(&paths.state_dir)?;
        let path = Self::path(paths);
        let raw = toml::to_string_pretty(self)?;
        std::fs::write(&path, raw).map_err(|source| ConfigError::Write { path, source })
    }

    pub fn add(&mut self, entry: IgnoredIncident) {
        if !self.incidents.contains(&entry) {
            self.incidents.push(entry);
        }
    }

    pub fn remove(&mut self, entry: &IgnoredIncident) {
        self.incidents.retain(|i| i != entry);
    }
}

#[cfg(test)]
mod tests {
    use sib_core::{IncidentKind, ServerId};

    use super::*;

    #[test]
    fn store_round_trips_through_toml() {
        let mut store = IgnoredStore::default();
        store.add(IgnoredIncident {
            server: ServerId::parse("neo").expect("id"),
            kind: IncidentKind::UnitFailed,
            subject: "fwupd-refresh.service".into(),
        });
        let raw = toml::to_string(&store).expect("toml");
        let parsed: IgnoredStore = toml::from_str(&raw).expect("parse");
        assert_eq!(parsed, store);
    }
}
