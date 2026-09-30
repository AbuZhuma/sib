use asiba_config::AppConfig;
use asiba_core::CustomCheck;

const ID_PREFIX: &str = "custom:";

#[derive(Clone, Debug, PartialEq)]
pub struct Draft {
    pub checks: Vec<CustomCheck>,
    pub editing: Option<String>,
}

impl Draft {
    pub fn from_config(config: &AppConfig) -> Self {
        Self {
            checks: config.checks.clone(),
            editing: None,
        }
    }

    pub fn is_dirty(&self, config: &AppConfig) -> bool {
        self.checks != config.checks
    }

    pub fn is_complete(&self) -> bool {
        self.checks.iter().all(CustomCheck::is_complete)
    }

    pub fn add(&mut self) {
        let id = self.free_id();
        self.editing = Some(id.clone());
        self.checks.push(CustomCheck::new(id));
    }

    pub fn remove(&mut self, id: &str) {
        self.checks.retain(|check| check.id != id);
        if self.editing.as_deref() == Some(id) {
            self.editing = None;
        }
    }

    pub fn toggle_editing(&mut self, id: &str) {
        self.editing = match self.editing.as_deref() {
            Some(current) if current == id => None,
            _ => Some(id.to_owned()),
        };
    }

    fn free_id(&self) -> String {
        (1..)
            .map(|index| format!("{ID_PREFIX}{index}"))
            .find(|id| self.checks.iter().all(|check| &check.id != id))
            .unwrap_or_default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn added_check_gets_a_free_id_and_opens_for_editing() {
        let mut draft = Draft::from_config(&AppConfig::default());
        draft.add();
        assert_eq!(draft.editing.as_deref(), Some("custom:1"));
        assert!(!draft.is_complete());
        draft.add();
        assert_eq!(draft.checks.len(), 2);
        assert_eq!(draft.editing.as_deref(), Some("custom:2"));
    }

    #[test]
    fn removing_the_edited_check_closes_the_editor() {
        let mut draft = Draft::from_config(&AppConfig::default());
        draft.add();
        draft.remove("custom:1");
        assert!(draft.checks.is_empty());
        assert!(draft.editing.is_none());
    }

    #[test]
    fn toggle_editing_opens_and_closes_the_same_check() {
        let mut draft = Draft::from_config(&AppConfig::default());
        draft.toggle_editing("custom:1");
        assert_eq!(draft.editing.as_deref(), Some("custom:1"));
        draft.toggle_editing("custom:1");
        assert!(draft.editing.is_none());
    }

    #[test]
    fn draft_without_changes_is_not_dirty() {
        let config = AppConfig::default();
        assert!(!Draft::from_config(&config).is_dirty(&config));
    }
}
