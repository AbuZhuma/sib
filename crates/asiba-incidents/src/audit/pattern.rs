use super::context::AuditContext;
use super::evidence::{Evidence, EvidenceSource};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Outcome {
    Pass,
    Warn,
    Fail,
    Skipped,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Area {
    Ssh,
    Access,
    Firewall,
    Kernel,
    Hardening,
    Resources,
    Reliability,
    Network,
    Updates,
    Logs,
    Collection,
}

impl Area {
    pub const ALL: [Area; 11] = [
        Area::Ssh,
        Area::Access,
        Area::Firewall,
        Area::Kernel,
        Area::Hardening,
        Area::Resources,
        Area::Reliability,
        Area::Network,
        Area::Updates,
        Area::Logs,
        Area::Collection,
    ];

    pub fn label(self) -> &'static str {
        match self {
            Self::Ssh => "SSH",
            Self::Access => "Доступ и права",
            Self::Firewall => "Сеть и файрвол",
            Self::Kernel => "Ядро",
            Self::Hardening => "Защита системы",
            Self::Resources => "Ресурсы",
            Self::Reliability => "Надёжность",
            Self::Network => "Сетевые интерфейсы",
            Self::Updates => "Обновления",
            Self::Logs => "Журнал",
            Self::Collection => "Сбор данных",
        }
    }

    pub fn is_security(self) -> bool {
        matches!(
            self,
            Self::Ssh | Self::Access | Self::Firewall | Self::Kernel | Self::Hardening
        )
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Weight {
    Low,
    Medium,
    High,
}

impl Weight {
    pub fn points(self) -> u32 {
        match self {
            Self::Low => 1,
            Self::Medium => 2,
            Self::High => 3,
        }
    }
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
