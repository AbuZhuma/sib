use chrono::{DateTime, Utc};
use sib_core::{AppState, Incident, IncidentDraft, ServerId, Severity};

use crate::detectors;

#[derive(Debug, Default)]
pub struct Reconciled {
    pub opened: Vec<Incident>,
    pub resolved: Vec<Incident>,
}

pub fn reconcile(state: &mut AppState, now: DateTime<Utc>) -> Reconciled {
    let drafts: Vec<(ServerId, Vec<IncidentDraft>)> = state
        .servers
        .values()
        .map(|server| {
            let drafts = detectors::detect(server, state)
                .into_iter()
                .filter(|d| d.severity >= Severity::Warning)
                .filter(|d| !state.is_incident_ignored(&server.spec.id, d.kind, &d.subject))
                .collect();
            (server.spec.id.clone(), drafts)
        })
        .collect();
    let mut result = Reconciled::default();
    for (server, drafts) in drafts {
        apply(state, &server, drafts, now, &mut result);
    }
    state.trim_resolved_incidents();
    result
}

fn apply(
    state: &mut AppState,
    server: &ServerId,
    drafts: Vec<IncidentDraft>,
    now: DateTime<Utc>,
    result: &mut Reconciled,
) {
    let mut matched = Vec::new();
    for draft in drafts {
        let existing = state
            .incidents
            .iter_mut()
            .find(|i| i.is_active() && &i.server == server && draft.same_condition(i));
        match existing {
            Some(incident) => {
                incident.severity = draft.severity;
                incident.summary = draft.summary;
                incident.evidence = draft.evidence;
                matched.push(incident.id);
            }
            None => {
                let id = next_id(state);
                let incident = Incident::open(id, server.clone(), draft, now);
                result.opened.push(incident.clone());
                state.incidents.push(incident);
                matched.push(id);
            }
        }
    }
    for incident in state
        .incidents
        .iter_mut()
        .filter(|i| i.is_active() && &i.server == server && !matched.contains(&i.id))
    {
        incident.resolved_at = Some(now);
        result.resolved.push(incident.clone());
    }
}

fn next_id(state: &AppState) -> u64 {
    state.incidents.iter().map(|i| i.id).max().unwrap_or(0) + 1
}

#[cfg(test)]
mod tests {
    use sib_core::{Alert, AuthMethod, ServerDescription, ServerSpec, ServerState, SudoMode};

    use super::*;

    fn state_with_server() -> AppState {
        let spec = ServerSpec {
            id: ServerId::parse("neo").expect("id"),
            host: "h".into(),
            port: 22,
            user: "u".into(),
            auth: AuthMethod::Auto,
            jump: None,
            sudo: SudoMode::None,
            description: ServerDescription::default(),
            location: None,
            modules: Default::default(),
            checks: Vec::new(),
            check_overrides: Default::default(),
            pipelines: Vec::new(),
        };
        let mut state = AppState::default();
        state
            .servers
            .insert(spec.id.clone(), ServerState::new(spec));
        state
    }

    fn alert(id: u64, severity: Severity) -> Alert {
        Alert {
            id,
            server: ServerId::parse("neo").expect("id"),
            rule_id: "cpu".into(),
            rule_name: "CPU".into(),
            metric: "cpu.total".into(),
            severity,
            value: 99.0,
            message: "high".into(),
            started_at: Utc::now(),
            resolved_at: None,
            acknowledged: false,
            muted_until: None,
        }
    }

    #[test]
    fn ignored_condition_never_opens_and_resolves_existing() {
        let mut state = state_with_server();
        state.alerts.push(alert(1, Severity::Critical));
        let now = Utc::now();
        let first = reconcile(&mut state, now);
        assert_eq!(first.opened.len(), 1);
        state
            .ignored_incidents
            .push(sib_core::IgnoredIncident::of(&first.opened[0]));
        let second = reconcile(&mut state, now);
        assert_eq!(second.resolved.len(), 1);
        assert_eq!(state.active_incidents().count(), 0);
        let third = reconcile(&mut state, now);
        assert!(third.opened.is_empty());
    }

    #[test]
    fn reconcile_opens_then_keeps_then_resolves_incident() {
        let mut state = state_with_server();
        state.alerts.push(alert(1, Severity::Critical));
        let now = Utc::now();
        let first = reconcile(&mut state, now);
        assert_eq!(first.opened.len(), 1);
        assert_eq!(first.opened[0].subject, "cpu");
        let second = reconcile(&mut state, now);
        assert!(second.opened.is_empty());
        assert_eq!(state.active_incidents().count(), 1);
        state.alerts[0].resolved_at = Some(now);
        let third = reconcile(&mut state, now);
        assert_eq!(third.resolved.len(), 1);
        assert_eq!(state.active_incidents().count(), 0);
    }

    #[test]
    fn reconcile_ignores_info_level_alerts() {
        let mut state = state_with_server();
        state.alerts.push(alert(1, Severity::Info));
        let result = reconcile(&mut state, Utc::now());
        assert!(result.opened.is_empty());
    }
}
