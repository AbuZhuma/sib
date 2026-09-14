use chrono::{DateTime, Utc};

#[derive(Debug, Clone, PartialEq)]
pub struct StoredSample {
    pub server: String,
    pub key: String,
    pub at: DateTime<Utc>,
    pub value: f64,
}
