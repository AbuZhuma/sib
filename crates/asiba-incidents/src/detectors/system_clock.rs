use asiba_core::{AppState, IncidentDraft, IncidentKind, ServerState, Severity};
use asiba_modules::system::{self, SystemInfo};

use super::Detector;

pub struct ClockDetector;

const OFFSET_WARNING_SECS: i64 = 60;

impl Detector for ClockDetector {
    fn detect(&self, server: &ServerState, _state: &AppState) -> Vec<IncidentDraft> {
        let Some(info) = server.data::<SystemInfo>(system::ID) else {
            return Vec::new();
        };
        if info.clock_offset_secs.abs() <= OFFSET_WARNING_SECS {
            return Vec::new();
        }
        vec![IncidentDraft::new(
            IncidentKind::Clock,
            Severity::Warning,
            "offset",
            format!(
                "Часы сервера расходятся с локальными на {} с",
                info.clock_offset_secs
            ),
        )]
    }
}
