use chrono::{DateTime, Utc};

use crate::module::ModuleId;
use crate::server::ServerId;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Danger {
    Normal,
    High,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ActionSpec {
    pub module: ModuleId,
    pub kind: &'static str,
    pub title: &'static str,
    pub danger: Danger,
}

impl ActionSpec {
    pub const fn new(
        module: ModuleId,
        kind: &'static str,
        title: &'static str,
        danger: Danger,
    ) -> Self {
        Self {
            module,
            kind,
            title,
            danger,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ActionRequest {
    pub kind: String,
    pub target: String,
    pub argument: Option<String>,
}

impl ActionRequest {
    pub fn new(kind: impl Into<String>, target: impl Into<String>) -> Self {
        Self {
            kind: kind.into(),
            target: target.into(),
            argument: None,
        }
    }

    pub fn with_argument(mut self, argument: impl Into<String>) -> Self {
        self.argument = Some(argument.into());
        self
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ActionOutcome {
    pub message: String,
}

impl ActionOutcome {
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ActionRecord {
    pub at: DateTime<Utc>,
    pub server: ServerId,
    pub module: String,
    pub kind: String,
    pub target: String,
    pub argument: Option<String>,
    pub is_success: bool,
    pub message: String,
}
