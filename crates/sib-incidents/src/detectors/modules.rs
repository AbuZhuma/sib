use sib_core::{AppState, IncidentDraft, IncidentKind, ServerState, Severity};

use super::Detector;

pub struct ModulesDetector;

impl Detector for ModulesDetector {
    fn detect(&self, server: &ServerState, _state: &AppState) -> Vec<IncidentDraft> {
        server
            .modules
            .iter()
            .filter_map(|(id, state)| {
                let error = state.last_error.as_ref()?;
                Some(IncidentDraft::new(
                    IncidentKind::ModuleError,
                    Severity::Warning,
                    id.0,
                    format!("Module {} collected no data: {error}", id.0),
                ))
            })
            .collect()
    }
}
