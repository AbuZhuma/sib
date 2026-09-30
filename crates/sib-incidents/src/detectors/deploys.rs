use sib_core::{AppState, IncidentDraft, IncidentKind, ServerState, Severity};
use sib_modules::deploy::{self, DeployState, DeployStatus};

use super::Detector;

pub struct DeploysDetector;

const EVIDENCE_LOG_LINES: usize = 10;

impl Detector for DeploysDetector {
    fn detect(&self, server: &ServerState, _state: &AppState) -> Vec<IncidentDraft> {
        let Some(state) = server.data::<DeployState>(deploy::ID) else {
            return Vec::new();
        };
        let mut seen = Vec::new();
        let mut drafts = Vec::new();
        for deploy in &state.snapshot.deploys {
            if seen.contains(&deploy.project) {
                continue;
            }
            seen.push(deploy.project.clone());
            if deploy.status != DeployStatus::Failed {
                continue;
            }
            let error = deploy
                .error
                .as_ref()
                .map(|e| e.line.clone())
                .unwrap_or_else(|| deploy.detail.clone());
            let start = deploy.log_tail.len().saturating_sub(EVIDENCE_LOG_LINES);
            drafts.push(
                IncidentDraft::new(
                    IncidentKind::DeployFailed,
                    Severity::Critical,
                    deploy.key.clone(),
                    format!("Деплой {} завершился ошибкой: {error}", deploy.project),
                )
                .evidence(deploy.log_tail[start..].iter().cloned()),
            );
        }
        drafts
    }
}
