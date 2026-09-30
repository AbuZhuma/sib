pub use sib_core::{Area, Weight};

use super::context::AuditContext;
use super::evidence::{Evidence, EvidenceSource};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Outcome {
    Pass,
    Warn,
    Fail,
    Skipped,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Verdict {
    pub outcome: Outcome,
    pub detail: String,
    pub instance: Option<String>,
    pub evidence: Option<Evidence>,
}

impl Verdict {
    pub fn new(outcome: Outcome, detail: impl Into<String>) -> Self {
        Self {
            outcome,
            detail: detail.into(),
            instance: None,
            evidence: None,
        }
    }

    pub fn pass(detail: impl Into<String>) -> Self {
        Self::new(Outcome::Pass, detail)
    }

    pub fn warn(detail: impl Into<String>) -> Self {
        Self::new(Outcome::Warn, detail)
    }

    pub fn fail(detail: impl Into<String>) -> Self {
        Self::new(Outcome::Fail, detail)
    }

    pub fn skipped(detail: impl Into<String>) -> Self {
        Self::new(Outcome::Skipped, detail)
    }

    pub fn graded(is_fail: bool, is_warn: bool, detail: impl Into<String>) -> Self {
        let outcome = if is_fail {
            Outcome::Fail
        } else if is_warn {
            Outcome::Warn
        } else {
            Outcome::Pass
        };
        Self::new(outcome, detail)
    }

    pub fn for_instance(mut self, instance: impl Into<String>) -> Self {
        self.instance = Some(instance.into());
        self
    }

    pub fn with_evidence(mut self, evidence: Evidence) -> Self {
        self.evidence = Some(evidence);
        self
    }

    pub fn single(self) -> Vec<Self> {
        vec![self]
    }
}

pub type Evaluate = fn(&AuditContext<'_>) -> Vec<Verdict>;

pub struct Pattern {
    pub id: &'static str,
    pub area: Area,
    pub subject: &'static str,
    pub description: &'static str,
    pub weight: Weight,
    pub advice: &'static str,
    pub evidence: EvidenceSource,
    pub evaluate: Evaluate,
}

impl Pattern {
    pub fn default_evidence(&self) -> Evidence {
        self.evidence.to_evidence()
    }
}
