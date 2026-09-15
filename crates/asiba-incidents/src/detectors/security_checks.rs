use asiba_core::{AppState, IncidentDraft, IncidentKind, ServerState, Severity};
use asiba_modules::security::{self, CheckStatus, SecuritySnapshot, Weight};

use super::Detector;

pub struct SecurityChecksDetector;

impl Detector for SecurityChecksDetector {
    fn detect(&self, server: &ServerState, _state: &AppState) -> Vec<IncidentDraft> {
        let Some(snapshot) = server.data::<SecuritySnapshot>(security::ID) else {
            return Vec::new();
        };
        security::checks(snapshot)
            .into_iter()
            .filter_map(|check| {
                let severity = match (check.status, check.weight) {
                    (CheckStatus::Fail, Weight::High) => Severity::Critical,
                    (CheckStatus::Fail, _) | (CheckStatus::Warn, Weight::High) => Severity::Warning,
                    (CheckStatus::Warn, _) => Severity::Info,
                    _ => return None,
                };
                Some(IncidentDraft::new(
                    IncidentKind::SecurityCheck,
                    severity,
                    check.label,
                    format!("{} — {}", check.label, check.detail),
                ))
            })
            .collect()
    }
}
