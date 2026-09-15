use asiba_core::{AppState, IncidentDraft, IncidentKind, ServerState, Severity};
use asiba_modules::anomalies::{self, AnomaliesSnapshot};

use super::Detector;

pub struct AnomaliesDetector;

impl Detector for AnomaliesDetector {
    fn detect(&self, server: &ServerState, _state: &AppState) -> Vec<IncidentDraft> {
        let Some(snapshot) = server.data::<AnomaliesSnapshot>(anomalies::ID) else {
            return Vec::new();
        };
        snapshot
            .signs
            .iter()
            .filter(|s| s.severity >= Severity::Warning)
            .map(|sign| {
                let mut evidence = vec![sign.detail.clone()];
                if !sign.peers.is_empty() {
                    evidence.push(format!("peers: {}", sign.peers.join(", ")));
                }
                IncidentDraft::new(
                    IncidentKind::Anomaly,
                    sign.severity,
                    format!("{:?}", sign.kind),
                    format!("{}: {}", sign.kind.label(), sign.detail),
                )
                .evidence(evidence)
            })
            .collect()
    }
}
