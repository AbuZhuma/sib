use sib_core::{Area, CheckOverride, CheckOverrides, CustomCheck, ServerState, Weight};

const ID_PREFIX: &str = "custom:";

#[derive(Clone, Debug, PartialEq)]
pub struct Draft {
    pub checks: Vec<CustomCheck>,
    pub overrides: CheckOverrides,
    pub editing: Option<String>,
    pub is_importing: bool,
    pub area: Option<Area>,
}

impl Draft {
    pub fn from_server(server: &ServerState) -> Self {
        Self {
            checks: server.spec.checks.clone(),
            overrides: server.spec.check_overrides.clone(),
            editing: None,
            is_importing: false,
            area: None,
        }
    }

    pub fn is_dirty(&self, server: &ServerState) -> bool {
        self.checks != server.spec.checks || self.saved_overrides() != server.spec.check_overrides
    }

    pub fn saved_overrides(&self) -> CheckOverrides {
        self.overrides
            .iter()
            .filter(|(_, value)| !value.is_default())
            .map(|(id, value)| (id.clone(), value.clone()))
            .collect()
    }

    pub fn is_complete(&self) -> bool {
        self.checks.iter().all(CustomCheck::is_complete)
    }

    pub fn is_enabled(&self, id: &str) -> bool {
        self.overrides.get(id).is_none_or(|value| value.enabled)
    }

    pub fn weight_of(&self, id: &str, default: Weight) -> Weight {
        self.overrides
            .get(id)
            .and_then(|value| value.weight)
            .unwrap_or(default)
    }

    pub fn is_overridden(&self, id: &str) -> bool {
        self.overrides.get(id).is_some_and(|v| !v.is_default())
    }

    pub fn set_enabled(&mut self, id: &str, enabled: bool) {
        self.entry(id).enabled = enabled;
    }

    pub fn set_weight(&mut self, id: &str, weight: Weight, default: Weight) {
        self.entry(id).weight = (weight != default).then_some(weight);
    }

    pub fn reset(&mut self, id: &str) {
        self.overrides.remove(id);
    }

    pub fn add(&mut self) {
        let id = self.free_id();
        self.editing = Some(id.clone());
        self.checks.push(CustomCheck::new(id));
    }

    pub fn import(&mut self, check: &CustomCheck) {
        let copy = CustomCheck {
            id: self.free_id(),
            ..check.clone()
        };
        self.editing = Some(copy.id.clone());
        self.checks.push(copy);
        self.is_importing = false;
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

    fn entry(&mut self, id: &str) -> &mut CheckOverride {
        self.overrides.entry(id.to_owned()).or_default()
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
    use sib_core::{AuthMethod, ServerId, ServerSpec, SudoMode};

    use super::*;

    fn server() -> ServerState {
        ServerState::new(ServerSpec {
            id: ServerId::parse("neo").expect("id"),
            host: "h".into(),
            port: 22,
            user: "root".into(),
            auth: AuthMethod::Auto,
            jump: None,
            sudo: SudoMode::None,
            description: Default::default(),
            location: None,
            modules: Default::default(),
            checks: Vec::new(),
            check_overrides: Default::default(),
        })
    }

    #[test]
    fn added_check_gets_a_free_id_and_opens_for_editing() {
        let mut draft = Draft::from_server(&server());
        draft.add();
        assert_eq!(draft.editing.as_deref(), Some("custom:1"));
        assert!(!draft.is_complete());
        draft.add();
        assert_eq!(draft.editing.as_deref(), Some("custom:2"));
    }

    #[test]
    fn imported_check_keeps_its_settings_but_gets_a_local_id() {
        let mut draft = Draft::from_server(&server());
        draft.add();
        let source = CustomCheck {
            name: "Backup".to_owned(),
            source: "backup --check".to_owned(),
            ..CustomCheck::new("custom:1")
        };
        draft.import(&source);
        let imported = draft.checks.last().expect("imported");
        assert_eq!(imported.id, "custom:2");
        assert_eq!(imported.name, "Backup");
        assert!(!draft.is_importing);
    }

    #[test]
    fn weight_equal_to_the_default_is_not_stored() {
        let server = server();
        let mut draft = Draft::from_server(&server);
        draft.set_weight("ssh.password_auth", Weight::High, Weight::High);
        assert!(draft.saved_overrides().is_empty());
        assert!(!draft.is_dirty(&server));
        draft.set_weight("ssh.password_auth", Weight::Low, Weight::High);
        assert_eq!(
            draft.weight_of("ssh.password_auth", Weight::High),
            Weight::Low
        );
        assert!(draft.is_dirty(&server));
    }

    #[test]
    fn disabled_check_is_stored_and_reset_removes_it() {
        let server = server();
        let mut draft = Draft::from_server(&server);
        draft.set_enabled("ssh.port", false);
        assert!(!draft.is_enabled("ssh.port"));
        assert_eq!(draft.saved_overrides().len(), 1);
        draft.reset("ssh.port");
        assert!(draft.is_enabled("ssh.port"));
        assert!(!draft.is_dirty(&server));
    }

    #[test]
    fn removing_the_edited_check_closes_the_editor() {
        let mut draft = Draft::from_server(&server());
        draft.add();
        draft.remove("custom:1");
        assert!(draft.checks.is_empty());
        assert!(draft.editing.is_none());
    }
}
