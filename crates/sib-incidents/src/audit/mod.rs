mod check;
mod context;
mod custom;
mod evidence;
mod pattern;
mod patterns;
mod score;

use sib_core::{AppState, ServerState};

pub use check::{AuditCheck, AuditCounts, SystemAudit};
pub use context::AuditContext;
pub use evidence::Evidence;
pub use pattern::{Area, Outcome, Pattern, Weight};
pub use score::{Grade, SecurityScore, security_score};

pub fn patterns() -> impl Iterator<Item = &'static Pattern> {
    patterns::all()
}

fn apply_overrides(checks: &mut Vec<AuditCheck>, overrides: &sib_core::CheckOverrides) {
    checks.retain(|check| overrides.get(&check.id).is_none_or(|value| value.enabled));
    for check in checks {
        if let Some(weight) = overrides.get(&check.id).and_then(|value| value.weight) {
            check.weight = weight;
        }
    }
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
    checks.extend(custom::checks(server));
    apply_overrides(&mut checks, &server.spec.check_overrides);
    checks.sort_by_key(|c| (c.area, std::cmp::Reverse(c.outcome)));
    SystemAudit { checks }
}

#[cfg(test)]
mod tests {
    use std::collections::HashSet;

    use sib_core::{AuthMethod, ServerDescription, ServerId, ServerSpec, SudoMode};

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
            checks: Vec::new(),
            check_overrides: Default::default(),
            pipelines: Vec::new(),
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
    fn disabled_pattern_is_dropped_and_weight_override_is_applied() {
        let mut server = bare_server("deploy");
        let id = patterns::all()
            .find(|p| p.area == Area::Collection)
            .map(|p| p.id.to_owned())
            .expect("pattern");
        server.spec.check_overrides.insert(
            id.clone(),
            sib_core::CheckOverride {
                enabled: true,
                weight: Some(Weight::Low),
            },
        );
        let audit = system_audit(&server, &AppState::default());
        assert!(
            audit
                .checks
                .iter()
                .any(|c| c.id == id && c.weight == Weight::Low)
        );
        server.spec.check_overrides.insert(
            id.clone(),
            sib_core::CheckOverride {
                enabled: false,
                weight: None,
            },
        );
        let audit = system_audit(&server, &AppState::default());
        assert!(audit.checks.iter().all(|c| c.id != id));
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
