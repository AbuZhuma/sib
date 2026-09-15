use asiba_core::{AppState, IncidentDraft, IncidentKind, ServerState, Severity};
use asiba_modules::memory::{self, MemorySnapshot};

use super::Detector;

pub struct MemoryDetector;

const SWAP_WARNING_PCT: f64 = 80.0;

impl Detector for MemoryDetector {
    fn detect(&self, server: &ServerState, _state: &AppState) -> Vec<IncidentDraft> {
        let Some(snapshot) = server.data::<MemorySnapshot>(memory::ID) else {
            return Vec::new();
        };
        let mut drafts = Vec::new();
        if snapshot.oom_kills > 0 {
            drafts.push(IncidentDraft::new(
                IncidentKind::Memory,
                Severity::Warning,
                "oom",
                format!(
                    "kernel OOM killer fired {} times since boot",
                    snapshot.oom_kills
                ),
            ));
        }
        if snapshot.swap_total_bytes > 0 && snapshot.swap_used_pct() >= SWAP_WARNING_PCT {
            drafts.push(IncidentDraft::new(
                IncidentKind::Memory,
                Severity::Warning,
                "swap",
                format!("swap is {:.0}% used", snapshot.swap_used_pct()),
            ));
        }
        drafts
    }
}
