use sib_core::{AppState, IncidentDraft, IncidentKind, ServerState, Severity};

use super::Detector;

pub struct AlertsDetector;

impl Detector for AlertsDetector {
    fn detect(&self, server: &ServerState, state: &AppState) -> Vec<IncidentDraft> {
        state
            .active_alerts()
            .filter(|a| a.server == server.spec.id && a.severity >= Severity::Warning)
            .map(|alert| {
                IncidentDraft::new(
                    IncidentKind::Alert,
                    alert.severity,
                    alert.rule_id.clone(),
                    format!("{}: {}", alert.rule_name, alert.message),
                )
                .evidence([format!("value: {:.2}", alert.value)])
            })
            .collect()
    }
}
