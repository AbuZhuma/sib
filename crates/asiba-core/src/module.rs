use std::fmt;
use std::time::Duration;

use async_trait::async_trait;
use serde::{Deserialize, Serialize};

use crate::action::{ActionOutcome, ActionRequest, ActionSpec};
use crate::snapshot::{CollectContext, Snapshot};
use crate::transport::{Transport, TransportError};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(transparent)]
pub struct ModuleId(pub &'static str);

impl fmt::Display for ModuleId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.0)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Schedule {
    Fast,
    Normal,
    Slow,
    OnDemand,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct Intervals {
    pub fast_secs: u64,
    pub normal_secs: u64,
    pub slow_secs: u64,
}

impl Default for Intervals {
    fn default() -> Self {
        Self {
            fast_secs: 2,
            normal_secs: 20,
            slow_secs: 300,
        }
    }
}

impl Intervals {
    pub fn clamped(self) -> Self {
        Self {
            fast_secs: self.fast_secs.clamp(1, 60),
            normal_secs: self.normal_secs.clamp(5, 600),
            slow_secs: self.slow_secs.clamp(30, 3600),
        }
    }
}

pub const SETTING_ENABLED: &str = "enabled";
pub const SETTING_INTERVAL: &str = "interval";

impl Schedule {
    pub fn interval(self) -> Option<Duration> {
        self.interval_with(&Intervals::default())
    }

    pub fn interval_with(self, intervals: &Intervals) -> Option<Duration> {
        let secs = match self {
            Self::Fast => intervals.fast_secs,
            Self::Normal => intervals.normal_secs,
            Self::Slow => intervals.slow_secs,
            Self::OnDemand => return None,
        };
        Some(Duration::from_secs(secs))
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Availability {
    Available,
    Partial { missing: Vec<String> },
    Unavailable { reason: String },
}

impl Availability {
    pub fn is_usable(&self) -> bool {
        !matches!(self, Self::Unavailable { .. })
    }
}

#[derive(Debug, Clone, thiserror::Error)]
pub enum ModuleError {
    #[error(transparent)]
    Transport(#[from] TransportError),
    #[error("не удалось разобрать вывод: {0}")]
    Parse(String),
    #[error("команда завершилась с ошибкой: {0}")]
    CommandFailed(String),
    #[error("модуль не поддерживает запрос {0}")]
    UnsupportedQuery(String),
    #[error("модуль не умеет подгружать историю")]
    UnsupportedBackfill,
    #[error("модуль не поддерживает действие {0}")]
    UnsupportedAction(String),
    #[error("действие не выполнено: {0}")]
    ActionFailed(String),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct QueryRequest {
    pub kind: String,
    pub target: String,
}

impl QueryRequest {
    pub fn new(kind: impl Into<String>, target: impl Into<String>) -> Self {
        Self {
            kind: kind.into(),
            target: target.into(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct QueryResponse {
    pub title: String,
    pub text: String,
}

#[async_trait]
pub trait Module: Send + Sync {
    fn id(&self) -> ModuleId;

    fn title(&self) -> &'static str;

    fn schedule(&self) -> Schedule;

    async fn detect(&self, transport: &dyn Transport) -> Result<Availability, ModuleError>;

    async fn collect(
        &self,
        transport: &dyn Transport,
        context: &CollectContext,
    ) -> Result<Snapshot, ModuleError>;

    async fn query(
        &self,
        _transport: &dyn Transport,
        request: &QueryRequest,
    ) -> Result<QueryResponse, ModuleError> {
        Err(ModuleError::UnsupportedQuery(request.kind.clone()))
    }

    async fn backfill(
        &self,
        _transport: &dyn Transport,
        _context: &CollectContext,
    ) -> Result<Snapshot, ModuleError> {
        Err(ModuleError::UnsupportedBackfill)
    }

    fn actions(&self) -> &'static [ActionSpec] {
        &[]
    }

    async fn perform(
        &self,
        _transport: &dyn Transport,
        request: &ActionRequest,
    ) -> Result<ActionOutcome, ModuleError> {
        Err(ModuleError::UnsupportedAction(request.kind.clone()))
    }
}
