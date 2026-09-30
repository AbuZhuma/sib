use asiba_core::{AppState, IncidentDraft, IncidentKind, ServerState, Severity};
use asiba_modules::updates::{self, UpdatesSnapshot};

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
                format!("Обновлений безопасности: {}", snapshot.security),
            ));
        }
        if snapshot.reboot_required {
            drafts.push(IncidentDraft::new(
                IncidentKind::Updates,
                Severity::Info,
                "reboot",
                "Требуется перезагрузка для применения обновлений",
            ));
        }
        drafts
    }
}
