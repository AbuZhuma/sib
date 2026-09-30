use sib_core::{AppState, IncidentDraft, IncidentKind, ServerState, Severity};
use sib_modules::updates::{self, UpdatesSnapshot};

use super::Detector;

pub struct UpdatesDetector;

impl Detector for UpdatesDetector {
    fn detect(&self, server: &ServerState, _state: &AppState) -> Vec<IncidentDraft> {
        let Some(snapshot) = server.data::<UpdatesSnapshot>(updates::ID) else {
            return Vec::new();
        };
        let mut drafts = Vec::new();
        if snapshot.security > 0 {
            drafts.push(IncidentDraft::new(
                IncidentKind::Updates,
                Severity::Warning,
                "security",
                format!("Security updates: {}", snapshot.security),
            ));
        }
        if snapshot.reboot_required {
            drafts.push(IncidentDraft::new(
                IncidentKind::Updates,
                Severity::Info,
                "reboot",
                "A reboot is needed to apply the updates",
            ));
        }
        drafts
    }
}
