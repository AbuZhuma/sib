use chrono::{DateTime, Utc};
use sib_core::{CustomCheck, Severity};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CheckOutcome {
    Passed,
    Failed,
    Skipped,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CheckResult {
    pub check: CustomCheck,
    pub outcome: CheckOutcome,
    pub exit_code: i32,
    pub output: String,
    pub at: DateTime<Utc>,
}

impl CheckResult {
    pub fn skipped(check: &CustomCheck, reason: impl Into<String>) -> Self {
        Self {
            check: check.clone(),
            outcome: CheckOutcome::Skipped,
            exit_code: 0,
            output: reason.into(),
            at: Utc::now(),
        }
    }

    pub fn is_failed(&self) -> bool {
        self.outcome == CheckOutcome::Failed
    }

    pub fn severity(&self) -> Severity {
        if self.is_failed() {
            self.check.weight.severity()
        } else {
            Severity::Info
        }
    }

    pub fn first_line(&self) -> &str {
        self.output.lines().next().unwrap_or_default().trim()
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ChecksSnapshot {
    pub results: Vec<CheckResult>,
}

impl ChecksSnapshot {
    pub fn find(&self, id: &str) -> Option<&CheckResult> {
        self.results.iter().find(|r| r.check.id == id)
    }

    pub fn failed(&self) -> impl Iterator<Item = &CheckResult> {
        self.results.iter().filter(|r| r.is_failed())
    }
}
