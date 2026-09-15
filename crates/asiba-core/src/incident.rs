use chrono::{DateTime, Utc};

use crate::{Severity, server::ServerId};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum IncidentKind {
    Alert,
    Anomaly,
    BruteForce,
    SecurityCheck,
    UnitFailed,
    ContainerDown,
    DeployFailed,
    DiskFull,
    Memory,
    Updates,
    ModuleError,
    Clock,
}

impl IncidentKind {
    pub fn key(self) -> &'static str {
        match self {
            Self::Alert => "alert",
            Self::Anomaly => "anomaly",
            Self::BruteForce => "brute_force",
            Self::SecurityCheck => "security_check",
            Self::UnitFailed => "unit_failed",
            Self::ContainerDown => "container_down",
            Self::DeployFailed => "deploy_failed",
            Self::DiskFull => "disk_full",
            Self::Memory => "memory",
            Self::Updates => "updates",
            Self::ModuleError => "module_error",
            Self::Clock => "clock",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IncidentDraft {
    pub kind: IncidentKind,
    pub severity: Severity,
    pub subject: String,
    pub summary: String,
    pub evidence: Vec<String>,
}

impl IncidentDraft {
    pub fn new(
        kind: IncidentKind,
        severity: Severity,
        subject: impl Into<String>,
        summary: impl Into<String>,
    ) -> Self {
        Self {
            kind,
            severity,
            subject: subject.into(),
            summary: summary.into(),
            evidence: Vec::new(),
        }
    }

    pub fn evidence(mut self, lines: impl IntoIterator<Item = String>) -> Self {
        self.evidence.extend(lines);
        self
    }

    pub fn same_condition(&self, other: &Incident) -> bool {
        self.kind == other.kind && self.subject == other.subject
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Incident {
    pub id: u64,
    pub server: ServerId,
    pub kind: IncidentKind,
    pub severity: Severity,
    pub subject: String,
    pub summary: String,
    pub evidence: Vec<String>,
    pub started_at: DateTime<Utc>,
    pub resolved_at: Option<DateTime<Utc>>,
}

impl Incident {
    pub fn open(id: u64, server: ServerId, draft: IncidentDraft, now: DateTime<Utc>) -> Self {
        Self {
            id,
            server,
            kind: draft.kind,
            severity: draft.severity,
            subject: draft.subject,
            summary: draft.summary,
            evidence: draft.evidence,
            started_at: now,
            resolved_at: None,
        }
    }

    pub fn is_active(&self) -> bool {
        self.resolved_at.is_none()
    }
}
