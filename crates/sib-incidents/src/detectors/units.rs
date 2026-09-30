use sib_core::{AppState, IncidentDraft, IncidentKind, ServerState, Severity};
use sib_modules::services::{self, ServicesSnapshot};

use super::Detector;

pub struct UnitsDetector;

impl Detector for UnitsDetector {
    fn detect(&self, server: &ServerState, _state: &AppState) -> Vec<IncidentDraft> {
        let Some(snapshot) = server.data::<ServicesSnapshot>(services::ID) else {
            return Vec::new();
        };
        snapshot
            .failed()
            .map(|unit| {
                IncidentDraft::new(
                    IncidentKind::UnitFailed,
                    Severity::Critical,
                    unit.name.clone(),
                    format!(
                        "Unit {} is failed (result {}, {} restarts)",
                        unit.name, unit.result, unit.restarts
                    ),
                )
                .evidence([
                    format!("description: {}", unit.description),
                    format!("fragment: {}", unit.fragment_path),
                ])
            })
            .collect()
    }
}
