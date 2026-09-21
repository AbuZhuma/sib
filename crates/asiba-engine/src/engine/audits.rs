use asiba_config::AiConfig;
use asiba_core::{AuditReport, AuditScope, AuditStatus, AuditTarget, IgnoredIncident};
use chrono::Utc;

use super::Engine;
use crate::ai::AuditJob;

impl Engine {
    pub(super) fn audit(&self, target: AuditTarget, scope: AuditScope, is_auto: bool) {
        let incident = match &scope {
            AuditScope::Incident { incident_id, .. } => self
                .state
                .read()
                .ok()
                .and_then(|s| s.incidents.iter().find(|i| i.id == *incident_id).cloned()),
            AuditScope::Full | AuditScope::Section { .. } => None,
        };
        let job = AuditJob {
            report_id: self.enqueue_report(&target, &scope),
            transport: target.server().and_then(|id| self.transport_of(id)),
            target,
            scope,
            incident,
            is_auto,
        };
        let _ = self.audits.send(Box::new(job));
    }

    pub(super) fn set_ignored(&self, ignored: Vec<IgnoredIncident>) {
        if let Ok(mut state) = self.state.write() {
            state.ignored_incidents = ignored;
            let now = Utc::now();
            let reconciled = asiba_incidents::reconcile(&mut state, now);
            tracing::info!(
                resolved = reconciled.resolved.len(),
                "список игнорируемых инцидентов обновлён"
            );
        }
        (self.notify)();
    }

    pub(super) fn cancel_audit(&self, id: u64) {
        if let Ok(mut state) = self.state.write()
            && state.audits.iter().any(|a| a.id == id && a.is_running())
        {
            state.audits.retain(|a| a.id != id);
            self.cancellations.cancel(id);
        }
        (self.notify)();
    }

    pub(super) fn enqueue_report(&self, target: &AuditTarget, scope: &AuditScope) -> u64 {
        let Ok(mut state) = self.state.write() else {
            return 0;
        };
        let id = state.audits.iter().map(|a| a.id).max().unwrap_or(0) + 1;
        state.audits.push(AuditReport {
            id,
            target: target.clone(),
            scope: scope.clone(),
            started_at: Utc::now(),
            finished_at: None,
            model: self.ai_config.borrow().model.clone(),
            context_tokens: 0,
            text: String::new(),
            status: AuditStatus::Queued,
        });
        drop(state);
        (self.notify)();
        id
    }

    pub(super) fn set_ai_config(&mut self, config: AiConfig) {
        let became_ready = config.is_ready() && !self.ai_config.borrow().is_ready();
        let _ = self.ai_config.send(config);
        let has_summary = self
            .state
            .read()
            .ok()
            .is_some_and(|s| s.audits.iter().any(|a| a.target == AuditTarget::Fleet));
        if became_ready && !has_summary {
            self.audit(AuditTarget::Fleet, AuditScope::Full, true);
        }
    }
}
