use chrono::{DateTime, Utc};

pub const MAX_DEPLOYS: usize = 50;
pub const ERROR_CONTEXT_LINES: usize = 20;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Source {
    Git,
    Compose,
    Systemd,
    LogFile,
    Runner,
}

impl Source {
    pub fn label(self) -> &'static str {
        match self {
            Self::Git => "git",
            Self::Compose => "compose",
            Self::Systemd => "systemd",
            Self::LogFile => "лог",
            Self::Runner => "CI runner",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DeployStatus {
    InProgress,
    Success,
    Failed,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StageStatus {
    Done,
    Active,
    Failed,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Stage {
    pub name: String,
    pub at: Option<DateTime<Utc>>,
    pub status: StageStatus,
}

impl Stage {
    pub fn done(name: impl Into<String>, at: Option<DateTime<Utc>>) -> Self {
        Self {
            name: name.into(),
            at,
            status: StageStatus::Done,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DeployError {
    pub line: String,
    pub context: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Deploy {
    pub key: String,
    pub project: String,
    pub source: Source,
    pub detail: String,
    pub started_at: DateTime<Utc>,
    pub finished_at: Option<DateTime<Utc>>,
    pub stages: Vec<Stage>,
    pub status: DeployStatus,
    pub error: Option<DeployError>,
    pub log_tail: Vec<String>,
}

impl Deploy {
    pub fn current_stage(&self) -> Option<&Stage> {
        self.stages
            .iter()
            .rev()
            .find(|s| s.status != StageStatus::Done)
            .or_else(|| self.stages.last())
    }

    pub fn duration_secs(&self, now: DateTime<Utc>) -> i64 {
        (self.finished_at.unwrap_or(now) - self.started_at)
            .num_seconds()
            .max(0)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RunnerKind {
    GithubActions,
    GitlabRunner,
}

impl RunnerKind {
    pub fn label(self) -> &'static str {
        match self {
            Self::GithubActions => "GitHub Actions runner",
            Self::GitlabRunner => "GitLab Runner",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Runner {
    pub kind: RunnerKind,
    pub working_dir: String,
    pub is_busy: bool,
    pub log_tail: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DeploySnapshot {
    pub deploys: Vec<Deploy>,
    pub runner: Option<Runner>,
    pub events_since: i64,
}

impl DeploySnapshot {
    pub fn active(&self) -> impl Iterator<Item = &Deploy> {
        self.deploys
            .iter()
            .filter(|d| d.status == DeployStatus::InProgress)
    }

    pub fn failed_count(&self) -> usize {
        self.deploys
            .iter()
            .filter(|d| d.status == DeployStatus::Failed)
            .count()
    }

    pub fn find(&self, key: &str) -> Option<&Deploy> {
        self.deploys.iter().find(|d| d.key == key)
    }
}
