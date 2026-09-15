use chrono::{DateTime, Utc};

use crate::incident::IncidentKind;
use crate::server::ServerId;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AuditTarget {
    Server(ServerId),
    Fleet,
}

impl AuditTarget {
    pub fn key(&self) -> String {
        match self {
            Self::Server(id) => id.to_string(),
            Self::Fleet => "fleet".to_owned(),
        }
    }

    pub fn server(&self) -> Option<&ServerId> {
        match self {
            Self::Server(id) => Some(id),
            Self::Fleet => None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AuditScope {
    Full,
    Incident {
        incident_id: u64,
        kind: IncidentKind,
        subject: String,
    },
    Section {
        key: String,
    },
}

impl AuditScope {
    pub fn key(&self) -> String {
        match self {
            Self::Full => "full".to_owned(),
            Self::Incident { kind, subject, .. } => format!("{}:{subject}", kind.key()),
            Self::Section { key } => format!("section:{key}"),
        }
    }

    pub fn is_incident(&self) -> bool {
        matches!(self, Self::Incident { .. })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AuditStatus {
    Queued,
    Running,
    Done,
    Failed(String),
    Cancelled,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AuditReport {
    pub id: u64,
    pub target: AuditTarget,
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
        matches!(self.status, AuditStatus::Queued | AuditStatus::Running)
    }

    pub fn is_done(&self) -> bool {
        self.status == AuditStatus::Done
    }

    pub fn is_for(&self, server: &ServerId) -> bool {
        self.target.server() == Some(server)
    }
}
