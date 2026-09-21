use super::evidence::Evidence;
use super::pattern::{Area, Outcome, Pattern, Verdict, Weight};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AuditCheck {
    pub id: &'static str,
    pub instance: Option<String>,
    pub area: Area,
    pub subject: &'static str,
    pub description: &'static str,
    pub weight: Weight,
    pub outcome: Outcome,
    pub detail: String,
    pub advice: &'static str,
    pub evidence: Evidence,
}

impl AuditCheck {
    pub fn from_verdict(pattern: &Pattern, verdict: Verdict) -> Self {
        Self {
            id: pattern.id,
            instance: verdict.instance,
            area: pattern.area,
            subject: pattern.subject,
            description: pattern.description,
            weight: pattern.weight,
            outcome: verdict.outcome,
            detail: verdict.detail,
            advice: pattern.advice,
            evidence: verdict
                .evidence
                .unwrap_or_else(|| pattern.default_evidence()),
        }
    }

    pub fn key(&self) -> String {
        match &self.instance {
            Some(instance) => format!("{}:{instance}", self.id),
            None => self.id.to_owned(),
        }
    }

    pub fn title(&self) -> String {
        match &self.instance {
            Some(instance) => format!("{} {instance}", self.subject),
            None => self.subject.to_owned(),
        }
    }

    pub fn is_problem(&self) -> bool {
        matches!(self.outcome, Outcome::Warn | Outcome::Fail)
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct AuditCounts {
    pub passed: usize,
    pub warnings: usize,
    pub failures: usize,
    pub skipped: usize,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct SystemAudit {
    pub checks: Vec<AuditCheck>,
}

impl SystemAudit {
    pub fn counts(&self) -> AuditCounts {
        let mut counts = AuditCounts::default();
        for check in &self.checks {
            match check.outcome {
                Outcome::Pass => counts.passed += 1,
                Outcome::Warn => counts.warnings += 1,
                Outcome::Fail => counts.failures += 1,
                Outcome::Skipped => counts.skipped += 1,
            }
        }
        counts
    }

    pub fn in_area(&self, area: Area) -> impl Iterator<Item = &AuditCheck> {
        self.checks.iter().filter(move |c| c.area == area)
    }

    pub fn security(&self) -> impl Iterator<Item = &AuditCheck> {
        self.checks.iter().filter(|c| c.area.is_security())
    }

    pub fn problems(&self) -> impl Iterator<Item = &AuditCheck> {
        self.checks.iter().filter(|c| c.is_problem())
    }

    pub fn find(&self, key: &str) -> Option<&AuditCheck> {
        self.checks.iter().find(|c| c.key() == key)
    }
}
