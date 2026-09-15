use std::path::{Path, PathBuf};

use asiba_core::{AuditReport, AuditStatus};
use chrono::Local;

pub fn write_report(dir: &Path, report: &AuditReport) -> Option<PathBuf> {
    let AuditStatus::Done = report.status else {
        return None;
    };
    let server_dir = dir.join(report.server.as_str());
    if let Err(error) = std::fs::create_dir_all(&server_dir) {
        tracing::warn!(path = %server_dir.display(), %error, "папка аудитов не создана");
        return None;
    }
    let stamp = report
        .started_at
        .with_timezone(&Local)
        .format("%Y-%m-%d_%H-%M-%S");
    let scope = report.scope.key().replace(['/', ':', ' '], "_");
    let path = server_dir.join(format!("{stamp}_{scope}.md"));
    let body = format!(
        "# Audit: {} — {}\n\nmodel: {}\ncontext_tokens: {}\nstarted: {}\n\n{}\n",
        report.server,
        report.scope.key(),
        report.model,
        report.context_tokens,
        report.started_at.format("%Y-%m-%d %H:%M:%S UTC"),
        report.text
    );
    match std::fs::write(&path, body) {
        Ok(()) => Some(path),
        Err(error) => {
            tracing::warn!(path = %path.display(), %error, "отчёт аудита не записан");
            None
        }
    }
}
