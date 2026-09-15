use std::sync::Arc;

use asiba_core::{AuditScope, Incident, ServerId, Transport};

#[derive(Clone)]
pub struct AuditJob {
    pub server: ServerId,
    pub scope: AuditScope,
    pub incident: Option<Incident>,
    pub transport: Option<Arc<dyn Transport>>,
    pub is_auto: bool,
}

impl AuditJob {
    pub fn cooldown_key(&self) -> String {
        format!("{}:{}", self.server, self.scope.key())
    }
}
