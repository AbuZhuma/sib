use asiba_core::{AppState, IncidentDraft, IncidentKind, ServerState, Severity};
use asiba_modules::disk::{self, DiskSnapshot};

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
        let mut drafts = Vec::new();
        for fs in &snapshot.filesystems {
            let used = fs.used_pct();
            let severity = if used >= CRITICAL_PCT {
                Severity::Critical
            } else if used >= WARNING_PCT {
                Severity::Warning
            } else {
                continue;
            };
            drafts.push(IncidentDraft::new(
                IncidentKind::DiskFull,
                severity,
                fs.mount.clone(),
                format!(
                    "{} заполнен на {used:.0} % (свободно {} МиБ)",
                    fs.mount,
                    fs.available_bytes / 1024 / 1024
                ),
            ));
        }
        for fs in &snapshot.filesystems {
            if fs.inodes_used_pct() >= INODES_WARNING_PCT {
                drafts.push(IncidentDraft::new(
                    IncidentKind::DiskFull,
                    Severity::Warning,
                    format!("{} inodes", fs.mount),
                    format!(
                        "inode на {} заняты на {:.0} %",
                        fs.mount,
                        fs.inodes_used_pct()
                    ),
                ));
            }
        }
        drafts
    }
}
