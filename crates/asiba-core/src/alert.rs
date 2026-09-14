use chrono::{DateTime, Duration, Utc};
use serde::{Deserialize, Serialize};

use crate::event::Severity;
use crate::server::ServerId;

pub const METRIC_OFFLINE: &str = "connection.offline";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Condition {
    Above,
    Below,
}

impl Condition {
    pub fn holds(self, value: f64, threshold: f64) -> bool {
        match self {
            Self::Above => value > threshold,
            Self::Below => value < threshold,
        }
    }

    pub fn symbol(self) -> &'static str {
        match self {
            Self::Above => ">",
            Self::Below => "<",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AlertRule {
    pub id: String,
    pub name: String,
    pub metric: String,
    pub condition: Condition,
    pub threshold: f64,
    pub for_secs: u64,
    pub severity: Severity,
    #[serde(default)]
    pub builtin: bool,
}

impl AlertRule {
    pub fn summary(&self) -> String {
        format!(
            "{} {} {} / {}с",
            self.metric,
            self.condition.symbol(),
            self.threshold,
            self.for_secs
        )
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Alert {
    pub id: u64,
    pub server: ServerId,
    pub rule_id: String,
    pub rule_name: String,
    pub severity: Severity,
    pub value: f64,
    pub message: String,
    pub started_at: DateTime<Utc>,
    pub resolved_at: Option<DateTime<Utc>>,
    pub acknowledged: bool,
    pub muted_until: Option<DateTime<Utc>>,
}

impl Alert {
    pub fn is_active(&self) -> bool {
        self.resolved_at.is_none()
    }

    pub fn is_muted(&self, now: DateTime<Utc>) -> bool {
        self.muted_until.is_some_and(|until| until > now)
    }

    pub fn duration(&self, now: DateTime<Utc>) -> Duration {
        self.resolved_at.unwrap_or(now) - self.started_at
    }
}
