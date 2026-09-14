use std::fmt;
use std::time::Duration;

use async_trait::async_trait;
use serde::{Deserialize, Serialize};

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

impl Schedule {
    pub fn interval(self) -> Option<Duration> {
        match self {
            Self::Fast => Some(Duration::from_secs(2)),
            Self::Normal => Some(Duration::from_secs(20)),
            Self::Slow => Some(Duration::from_secs(300)),
            Self::OnDemand => None,
        }
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
}
