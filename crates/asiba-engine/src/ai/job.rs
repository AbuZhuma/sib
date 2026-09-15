use std::sync::Arc;

use asiba_core::{AuditScope, AuditTarget, Incident, Transport};

#[derive(Clone)]
pub struct AuditJob {
    pub target: AuditTarget,
    pub scope: AuditScope,
    pub incident: Option<Incident>,
    pub transport: Option<Arc<dyn Transport>>,
    pub is_auto: bool,
}

impl AuditJob {
    pub fn cooldown_key(&self) -> String {
        format!("{}:{}", self.target.key(), self.scope.key())
    }
}
