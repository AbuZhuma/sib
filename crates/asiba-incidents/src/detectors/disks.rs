use asiba_core::{AppState, IncidentDraft, IncidentKind, ServerState, Severity};
use asiba_modules::disk::{self, DiskSnapshot, Filesystem};

use super::Detector;

pub struct DisksDetector;

const CRITICAL_PCT: f64 = 90.0;
const WARNING_PCT: f64 = 80.0;
const INODES_WARNING_PCT: f64 = 90.0;

impl Detector for DisksDetector {
    fn detect(&self, server: &ServerState, _state: &AppState) -> Vec<IncidentDraft> {
        let Some(snapshot) = server.data::<DiskSnapshot>(disk::ID) else {
            return Vec::new();
        };
        let space = snapshot.filesystems.iter().filter_map(space_draft);
        let inodes = snapshot.filesystems.iter().filter_map(inodes_draft);
        space.chain(inodes).collect()
    }
}

fn space_draft(fs: &Filesystem) -> Option<IncidentDraft> {
    let used = fs.used_pct();
    let severity = if used >= CRITICAL_PCT {
        Severity::Critical
    } else if used >= WARNING_PCT {
        Severity::Warning
    } else {
        return None;
    };
    Some(IncidentDraft::new(
        IncidentKind::DiskFull,
        severity,
        fs.mount.clone(),
        format!(
            "{} заполнен на {used:.0} % (свободно {} МиБ)",
            fs.mount,
            fs.available_bytes / 1024 / 1024
        ),
    ))
}

fn inodes_draft(fs: &Filesystem) -> Option<IncidentDraft> {
    let used = fs.inodes_used_pct();
    if used < INODES_WARNING_PCT {
        return None;
    }
    Some(IncidentDraft::new(
        IncidentKind::DiskFull,
        Severity::Warning,
        format!("{} inodes", fs.mount),
        format!("inode на {} заняты на {used:.0} %", fs.mount),
    ))
}
