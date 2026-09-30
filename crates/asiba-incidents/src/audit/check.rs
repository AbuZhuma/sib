use super::evidence::Evidence;
use super::pattern::{Area, Outcome, Pattern, Verdict, Weight};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AuditCheck {
    pub id: String,
    pub instance: Option<String>,
    pub area: Area,
    pub subject: String,
    pub description: String,
    pub weight: Weight,
    pub outcome: Outcome,
    pub detail: String,
    pub advice: String,
    pub evidence: Evidence,
}

impl AuditCheck {
    pub fn from_verdict(pattern: &Pattern, verdict: Verdict) -> Self {
        Self {
            id: pattern.id.to_owned(),
            instance: verdict.instance,
            area: pattern.area,
            subject: pattern.subject.to_owned(),
            description: pattern.description.to_owned(),
            weight: pattern.weight,
            outcome: verdict.outcome,
            detail: verdict.detail,
            advice: pattern.advice.to_owned(),
            evidence: verdict
                .evidence
                .unwrap_or_else(|| pattern.default_evidence()),
        }
    }

    pub fn key(&self) -> String {
        match &self.instance {
            Some(instance) => format!("{}:{instance}", self.id),
            None => self.id.clone(),
        }
    }

    pub fn title(&self) -> String {
        match &self.instance {
            Some(instance) => format!("{} {instance}", self.subject),
            None => self.subject.clone(),
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

    pub fn alerting(&self) -> impl Iterator<Item = &AuditCheck> {
        self.checks
            .iter()
            .filter(|c| c.area.is_security() || c.area == Area::Custom)
    }

    pub fn problems(&self) -> impl Iterator<Item = &AuditCheck> {
        self.checks.iter().filter(|c| c.is_problem())
    }

    pub fn find(&self, key: &str) -> Option<&AuditCheck> {
        self.checks.iter().find(|c| c.key() == key)
    }
}
