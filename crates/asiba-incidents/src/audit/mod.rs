mod network;
mod reliability;
mod resources;
mod security;
mod system;

use asiba_core::{AppState, ServerState};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Outcome {
    Pass,
    Warn,
    Fail,
    Skipped,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Area {
    Security,
    Resources,
    Reliability,
    Network,
    Updates,
    Logs,
    Collection,
}

impl Area {
    pub const ALL: [Area; 7] = [
        Area::Security,
        Area::Resources,
        Area::Reliability,
        Area::Network,
        Area::Updates,
        Area::Logs,
        Area::Collection,
    ];

    pub fn label(self) -> &'static str {
        match self {
            Self::Security => "Безопасность",
            Self::Resources => "Ресурсы",
            Self::Reliability => "Надёжность",
            Self::Network => "Сеть",
            Self::Updates => "Обновления",
            Self::Logs => "Журнал",
            Self::Collection => "Сбор данных",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AuditCheck {
    pub area: Area,
    pub title: String,
    pub outcome: Outcome,
    pub detail: String,
    pub advice: &'static str,
}

impl AuditCheck {
    pub fn new(area: Area, title: impl Into<String>, advice: &'static str) -> Self {
        Self {
            area,
            title: title.into(),
            outcome: Outcome::Skipped,
            detail: String::new(),
            advice,
        }
    }

    pub fn outcome(mut self, outcome: Outcome, detail: impl Into<String>) -> Self {
        self.outcome = outcome;
        self.detail = detail.into();
        self
    }

    pub fn graded(self, is_fail: bool, is_warn: bool, detail: impl Into<String>) -> Self {
        let outcome = if is_fail {
            Outcome::Fail
        } else if is_warn {
            Outcome::Warn
        } else {
            Outcome::Pass
        };
        self.outcome(outcome, detail)
    }

    pub fn is_problem(&self) -> bool {
        matches!(self.outcome, Outcome::Warn | Outcome::Fail)
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct AuditCounts {
    pub passed: usize,
    pub warnings: usize,
    pub failures: usize,
    pub skipped: usize,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct SystemAudit {
    pub checks: Vec<AuditCheck>,
}

impl SystemAudit {
    pub fn counts(&self) -> AuditCounts {
        let mut counts = AuditCounts::default();
        for check in &self.checks {
            match check.outcome {
                Outcome::Pass => counts.passed += 1,
                Outcome::Warn => counts.warnings += 1,
                Outcome::Fail => counts.failures += 1,
                Outcome::Skipped => counts.skipped += 1,
            }
        }
        counts
    }

    pub fn in_area(&self, area: Area) -> impl Iterator<Item = &AuditCheck> {
        self.checks.iter().filter(move |c| c.area == area)
    }

    pub fn problems(&self) -> impl Iterator<Item = &AuditCheck> {
        self.checks.iter().filter(|c| c.is_problem())
    }
}

pub fn system_audit(server: &ServerState, state: &AppState) -> SystemAudit {
    let mut checks = Vec::new();
    checks.extend(security::checks(server, state));
    checks.extend(resources::checks(server));
    checks.extend(reliability::checks(server));
    checks.extend(network::checks(server));
    checks.extend(system::checks(server));
    checks.sort_by_key(|c| (c.area, std::cmp::Reverse(c.outcome)));
    SystemAudit { checks }
}

#[cfg(test)]
mod tests {
    use asiba_core::{AuthMethod, ServerDescription, ServerId, ServerSpec, SudoMode};

    use super::*;

    fn bare_server(user: &str) -> ServerState {
        ServerState::new(ServerSpec {
            id: ServerId::parse("neo").expect("id"),
            host: "h".into(),
            port: 22,
            user: user.into(),
            auth: AuthMethod::Auto,
            jump: None,
            sudo: SudoMode::None,
            description: ServerDescription::default(),
            location: None,
            modules: Default::default(),
        })
    }

    #[test]
    fn graded_maps_flags_to_outcome_in_priority_order() {
        let check = AuditCheck::new(Area::Logs, "t", "a");
        assert_eq!(check.clone().graded(true, true, "").outcome, Outcome::Fail);
        assert_eq!(check.clone().graded(false, true, "").outcome, Outcome::Warn);
        assert_eq!(check.graded(false, false, "").outcome, Outcome::Pass);
    }

    #[test]
    fn bare_server_without_sudo_gets_collection_warning_only() {
        let audit = system_audit(&bare_server("deploy"), &AppState::default());
        let counts = audit.counts();
        assert_eq!(counts.warnings, 1);
        assert_eq!(counts.failures, 0);
        assert!(audit.problems().all(|c| c.area == Area::Collection));
        let root = system_audit(&bare_server("root"), &AppState::default());
        assert_eq!(root.counts().warnings, 0);
    }
}
