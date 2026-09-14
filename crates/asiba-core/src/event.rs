use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::module::ModuleId;
use crate::server::ServerId;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Severity {
    Info,
    Warning,
    Critical,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Event {
    pub at: DateTime<Utc>,
    pub server: Option<ServerId>,
    pub module: ModuleId,
    pub severity: Severity,
    pub message: String,
}

impl Event {
    pub fn new(module: ModuleId, severity: Severity, message: impl Into<String>) -> Self {
        Self {
            at: Utc::now(),
            server: None,
            module,
            severity,
            message: message.into(),
        }
    }

    pub fn for_server(mut self, server: ServerId) -> Self {
        self.server = Some(server);
        self
    }
}
