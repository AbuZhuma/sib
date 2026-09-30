use asiba_core::{CustomCheck, ServerState};

const ID_PREFIX: &str = "custom:";

#[derive(Clone, Debug, PartialEq)]
pub struct Draft {
    pub checks: Vec<CustomCheck>,
    pub editing: Option<String>,
    pub is_importing: bool,
}

impl Draft {
    pub fn from_server(server: &ServerState) -> Self {
        Self {
            checks: server.spec.checks.clone(),
            editing: None,
            is_importing: false,
        }
    }

    pub fn is_dirty(&self, server: &ServerState) -> bool {
        self.checks != server.spec.checks
    }

    pub fn is_complete(&self) -> bool {
        self.checks.iter().all(CustomCheck::is_complete)
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

    fn free_id(&self) -> String {
        (1..)
            .map(|index| format!("{ID_PREFIX}{index}"))
            .find(|id| self.checks.iter().all(|check| &check.id != id))
            .unwrap_or_default()
    }
}

#[cfg(test)]
mod tests {
    use asiba_core::{AuthMethod, ServerId, ServerSpec, SudoMode};

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
            name: "Бэкап".to_owned(),
            source: "backup --check".to_owned(),
            ..CustomCheck::new("custom:1")
        };
        draft.import(&source);
        let imported = draft.checks.last().expect("imported");
        assert_eq!(imported.id, "custom:2");
        assert_eq!(imported.name, "Бэкап");
        assert!(!draft.is_importing);
    }

    #[test]
    fn removing_the_edited_check_closes_the_editor() {
        let mut draft = Draft::from_server(&server());
        draft.add();
        draft.remove("custom:1");
        assert!(draft.checks.is_empty());
        assert!(draft.editing.is_none());
    }

    #[test]
    fn draft_without_changes_is_not_dirty() {
        let server = server();
        assert!(!Draft::from_server(&server).is_dirty(&server));
    }
}
