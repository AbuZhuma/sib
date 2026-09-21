mod check;
mod context;
mod evidence;
mod pattern;
mod patterns;
mod score;

use asiba_core::{AppState, ServerState};

pub use check::{AuditCheck, AuditCounts, SystemAudit};
pub use context::AuditContext;
pub use evidence::Evidence;
pub use pattern::{Area, Outcome, Pattern, Weight};
pub use score::{Grade, SecurityScore, security_score};

pub fn patterns() -> impl Iterator<Item = &'static Pattern> {
    patterns::all()
}

pub fn system_audit(server: &ServerState, state: &AppState) -> SystemAudit {
    let context = AuditContext { server, state };
    let mut checks: Vec<AuditCheck> = patterns::all()
        .flat_map(|pattern| {
            (pattern.evaluate)(&context)
                .into_iter()
                .map(|verdict| AuditCheck::from_verdict(pattern, verdict))
        })
        .collect();
    checks.sort_by_key(|c| (c.area, std::cmp::Reverse(c.outcome)));
    SystemAudit { checks }
}

#[cfg(test)]
mod tests {
    use std::collections::HashSet;

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
    fn pattern_ids_are_unique() {
        let ids: HashSet<&str> = patterns::all().map(|p| p.id).collect();
        assert_eq!(ids.len(), patterns::all().count());
    }

    #[test]
    fn every_pattern_has_texts() {
        for pattern in patterns::all() {
            assert!(!pattern.subject.is_empty(), "{}", pattern.id);
            assert!(!pattern.description.is_empty(), "{}", pattern.id);
            assert!(!pattern.advice.is_empty(), "{}", pattern.id);
        }
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
