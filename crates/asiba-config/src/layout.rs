use std::collections::BTreeMap;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use crate::error::ConfigError;
use crate::paths::Paths;

const LAYOUT_FILE: &str = "layout.toml";

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct SummaryLayout {
    pub order: Vec<String>,
    pub hidden: Vec<String>,
}

impl SummaryLayout {
    pub fn is_hidden(&self, module: &str) -> bool {
        self.hidden.iter().any(|m| m == module)
    }

    pub fn toggle_hidden(&mut self, module: &str) {
        if self.is_hidden(module) {
            self.hidden.retain(|m| m != module);
        } else {
            self.hidden.push(module.to_owned());
        }
    }

    pub fn arrange<'a>(&self, modules: &[&'a str]) -> Vec<&'a str> {
        let mut ordered: Vec<&str> = self
            .order
            .iter()
            .filter_map(|wanted| modules.iter().copied().find(|m| m == wanted))
            .collect();
        let rest: Vec<&str> = modules
            .iter()
            .copied()
            .filter(|m| !ordered.contains(m))
            .collect();
        ordered.extend(rest);
        ordered
    }

    pub fn shift(&mut self, modules: &[&str], module: &str, delta: isize) {
        let mut order: Vec<String> = self
            .arrange(modules)
            .into_iter()
            .map(str::to_owned)
            .collect();
        let Some(index) = order.iter().position(|m| m == module) else {
            return;
        };
        let target = index as isize + delta;
        if target < 0 || target >= order.len() as isize {
            return;
        }
        order.swap(index, target as usize);
        self.order = order;
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct LayoutStore {
    pub servers: BTreeMap<String, SummaryLayout>,
}

impl LayoutStore {
    fn path(paths: &Paths) -> PathBuf {
        paths.state_dir.join(LAYOUT_FILE)
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

    pub fn for_server(&self, server: &str) -> SummaryLayout {
        self.servers.get(server).cloned().unwrap_or_default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn arrange_keeps_saved_order_and_appends_new_modules() {
        let layout = SummaryLayout {
            order: vec!["cpu".into(), "memory".into(), "gone".into()],
            hidden: Vec::new(),
        };
        let arranged = layout.arrange(&["memory", "disk", "cpu"]);
        assert_eq!(arranged, vec!["cpu", "memory", "disk"]);
    }

    #[test]
    fn shift_moves_module_and_clamps_at_edges() {
        let mut layout = SummaryLayout::default();
        layout.shift(&["a", "b", "c"], "c", -1);
        assert_eq!(layout.order, vec!["a", "c", "b"]);
        layout.shift(&["a", "b", "c"], "a", -1);
        assert_eq!(layout.order, vec!["a", "c", "b"]);
    }

    #[test]
    fn toggle_hidden_flips_state() {
        let mut layout = SummaryLayout::default();
        layout.toggle_hidden("cpu");
        assert!(layout.is_hidden("cpu"));
        layout.toggle_hidden("cpu");
        assert!(!layout.is_hidden("cpu"));
    }
}
