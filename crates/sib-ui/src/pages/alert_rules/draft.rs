use sib_config::AppConfig;
use sib_core::{AlertRule, Condition, Severity};

const CUSTOM_PREFIX: &str = "custom:";
const NEW_FOR_SECS: u64 = 60;

#[derive(Clone, Debug, PartialEq)]
pub struct Draft {
    pub rules: Vec<AlertRule>,
    pub notifications: bool,
}

impl Draft {
    pub fn from_config(config: &AppConfig) -> Self {
        let defaults = sib_alerts::builtin_rules();
        let rules = sib_alerts::merge_rules(&config.alert_rules)
            .into_iter()
            .map(|mut rule| {
                rule.builtin = defaults.iter().any(|d| d.id == rule.id);
                rule
            })
            .collect();
        Self {
            rules,
            notifications: config.desktop_notifications,
        }
    }

    pub fn saved_rules(&self) -> Vec<AlertRule> {
        let defaults = sib_alerts::builtin_rules();
        self.rules
            .iter()
            .filter(|rule| !defaults.contains(rule))
            .map(|rule| AlertRule {
                builtin: false,
                ..rule.clone()
            })
            .collect()
    }

    pub fn is_complete(&self) -> bool {
        self.rules
            .iter()
            .all(|rule| !rule.name.trim().is_empty() && !rule.metric.trim().is_empty())
    }

    pub fn add(&mut self) {
        self.rules.push(AlertRule {
            id: self.free_id(),
            name: String::new(),
            metric: String::new(),
            condition: Condition::Above,
            threshold: 0.0,
            for_secs: NEW_FOR_SECS,
            severity: Severity::Warning,
            enabled: true,
            builtin: false,
        });
    }

    pub fn reset(&mut self, id: &str) {
        let Some(default) = sib_alerts::builtin_rules()
            .into_iter()
            .find(|rule| rule.id == id)
        else {
            return;
        };
        for rule in &mut self.rules {
            if rule.id == id {
                *rule = default.clone();
            }
        }
    }

    pub fn remove(&mut self, id: &str) {
        self.rules.retain(|rule| rule.id != id);
    }

    fn free_id(&self) -> String {
        (1..)
            .map(|index| format!("{CUSTOM_PREFIX}{index}"))
            .find(|id| self.rules.iter().all(|rule| &rule.id != id))
            .unwrap_or_default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn draft() -> Draft {
        Draft::from_config(&AppConfig::default())
    }

    #[test]
    fn untouched_builtin_rules_are_not_saved() {
        assert!(draft().saved_rules().is_empty());
    }

    #[test]
    fn edited_builtin_rule_is_saved_and_reset_removes_it() {
        let mut draft = draft();
        let id = draft.rules[1].id.clone();
        draft.rules[1].threshold = 55.0;
        let saved = draft.saved_rules();
        assert_eq!(saved.len(), 1);
        assert_eq!(saved[0].id, id);
        draft.reset(&id);
        assert!(draft.saved_rules().is_empty());
    }

    #[test]
    fn added_rule_gets_a_free_id_and_blocks_apply_until_filled() {
        let mut draft = draft();
        draft.add();
        assert_eq!(draft.rules.last().map(|r| r.id.as_str()), Some("custom:1"));
        assert!(!draft.is_complete());
        draft.add();
        assert_eq!(draft.rules.last().map(|r| r.id.as_str()), Some("custom:2"));
    }

    #[test]
    fn disabled_builtin_rule_is_saved_as_override() {
        let mut draft = draft();
        draft.rules[0].enabled = false;
        assert_eq!(draft.saved_rules().len(), 1);
    }
}
