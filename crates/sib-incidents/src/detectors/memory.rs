use chrono::Utc;
use sib_core::{AppState, IncidentDraft, IncidentKind, ServerState, Severity};
use sib_modules::memory::{self, MemorySnapshot};

use super::Detector;

pub struct MemoryDetector;

const SWAP_WARNING_PCT: f64 = 80.0;

impl Detector for MemoryDetector {
    fn detect(&self, server: &ServerState, _state: &AppState) -> Vec<IncidentDraft> {
        let Some(snapshot) = server.data::<MemorySnapshot>(memory::ID) else {
            return Vec::new();
        };
        let mut drafts = Vec::new();
        let recent = snapshot.recent_oom_kills(Utc::now());
        if recent > 0 {
            drafts.push(IncidentDraft::new(
                IncidentKind::Memory,
                Severity::Warning,
                "oom",
                format!("OOM killer fired {recent} times in a day"),
            ));
        }
        if snapshot.swap_total_bytes > 0 && snapshot.swap_used_pct() >= SWAP_WARNING_PCT {
            drafts.push(IncidentDraft::new(
                IncidentKind::Memory,
                Severity::Warning,
                "swap",
                format!("Swap is {:.0} % used", snapshot.swap_used_pct()),
            ));
        }
        drafts
    }
}
