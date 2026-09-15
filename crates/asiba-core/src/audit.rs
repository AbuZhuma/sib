use chrono::{DateTime, Utc};

use crate::incident::IncidentKind;
use crate::server::ServerId;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AuditScope {
    Full,
    Incident {
        incident_id: u64,
        kind: IncidentKind,
        subject: String,
    },
}

impl AuditScope {
    pub fn key(&self) -> String {
        match self {
            Self::Full => "full".to_owned(),
            Self::Incident { kind, subject, .. } => format!("{}:{subject}", kind.key()),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AuditStatus {
    Running,
    Done,
    Failed(String),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AuditReport {
    pub id: u64,
    pub server: ServerId,
    pub scope: AuditScope,
    pub started_at: DateTime<Utc>,
    pub finished_at: Option<DateTime<Utc>>,
    pub model: String,
    pub context_tokens: usize,
    pub text: String,
    pub status: AuditStatus,
}

impl AuditReport {
    pub fn is_running(&self) -> bool {
        self.status == AuditStatus::Running
    }
}
