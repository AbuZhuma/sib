use sib_core::{AppState, IncidentDraft, IncidentKind, ServerState, Severity};

use super::Detector;
use crate::audit::{Outcome, Weight, system_audit};

const COVERED_BY_OWN_DETECTOR: [&str; 2] = ["firewall.brute_force", "firewall.unbanned_attackers"];

pub struct SecurityChecksDetector;

impl Detector for SecurityChecksDetector {
    fn detect(&self, server: &ServerState, state: &AppState) -> Vec<IncidentDraft> {
        system_audit(server, state)
            .alerting()
            .filter(|check| !COVERED_BY_OWN_DETECTOR.contains(&check.id.as_str()))
            .filter_map(|check| {
                let severity = match (check.outcome, check.weight) {
                    (Outcome::Fail, Weight::High) => Severity::Critical,
                    (Outcome::Fail, _) | (Outcome::Warn, Weight::High) => Severity::Warning,
                    (Outcome::Warn, _) => Severity::Info,
                    _ => return None,
                };
                Some(IncidentDraft::new(
                    IncidentKind::SecurityCheck,
                    severity,
                    check.key(),
                    format!("{} - {}", check.title(), check.detail),
                ))
            })
            .collect()
    }
}
