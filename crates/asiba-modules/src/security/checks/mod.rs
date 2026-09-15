mod hardening_checks;
mod ssh_checks;

use super::model::SecuritySnapshot;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CheckStatus {
    Pass,
    Warn,
    Fail,
    Unknown,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Category {
    Ssh,
    Access,
    Network,
    Kernel,
    System,
}

impl Category {
    pub const ALL: [Category; 5] = [
        Category::Ssh,
        Category::Access,
        Category::Network,
        Category::Kernel,
        Category::System,
    ];

    pub fn label(self) -> &'static str {
        match self {
            Self::Ssh => "SSH",
            Self::Access => "Доступ и права",
            Self::Network => "Сеть и файрвол",
            Self::Kernel => "Ядро",
            Self::System => "Система",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Weight {
    Low,
    Medium,
    High,
}

impl Weight {
    fn points(self) -> u32 {
        match self {
            Self::Low => 1,
            Self::Medium => 2,
            Self::High => 3,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Check {
    pub category: Category,
    pub label: &'static str,
    pub detail: String,
    pub status: CheckStatus,
    pub weight: Weight,
}

impl Check {
    pub fn new(category: Category, label: &'static str, weight: Weight) -> Self {
        Self {
            category,
            label,
            detail: String::new(),
            status: CheckStatus::Unknown,
            weight,
        }
    }

    pub fn with(mut self, status: CheckStatus, detail: impl Into<String>) -> Self {
        self.status = status;
        self.detail = detail.into();
        self
    }

    pub fn is_problem(&self) -> bool {
        matches!(self.status, CheckStatus::Warn | CheckStatus::Fail)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Grade {
    A,
    B,
    C,
    D,
    F,
}

impl Grade {
    pub fn letter(self) -> &'static str {
        match self {
            Self::A => "A",
            Self::B => "B",
            Self::C => "C",
            Self::D => "D",
            Self::F => "F",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Score {
    pub grade: Grade,
    pub percent: u32,
    pub passed: usize,
    pub known: usize,
    pub failed_high: usize,
}

pub fn checks(snapshot: &SecuritySnapshot) -> Vec<Check> {
    let mut checks = ssh_checks::all(snapshot);
    checks.extend(hardening_checks::all(snapshot));
    checks
}

pub fn score(checks: &[Check]) -> Score {
    let known: Vec<&Check> = checks
        .iter()
        .filter(|c| c.status != CheckStatus::Unknown)
        .collect();
    let total: u32 = known.iter().map(|c| c.weight.points()).sum();
    let earned: u32 = known
        .iter()
        .map(|c| match c.status {
            CheckStatus::Pass => c.weight.points(),
            CheckStatus::Warn => c.weight.points() / 2,
            _ => 0,
        })
        .sum();
    let percent = (earned * 100).checked_div(total).unwrap_or(0);
    let failed_high = known
        .iter()
        .filter(|c| c.status == CheckStatus::Fail && c.weight == Weight::High)
        .count();
    let grade = match (percent, failed_high) {
        (90.., 0) => Grade::A,
        (75.., 0) => Grade::B,
        (60.., _) => Grade::C,
        (40.., _) => Grade::D,
        _ => Grade::F,
    };
    Score {
        grade,
        percent,
        passed: known
            .iter()
            .filter(|c| c.status == CheckStatus::Pass)
            .count(),
        known: known.len(),
        failed_high,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn check(status: CheckStatus, weight: Weight) -> Check {
        Check::new(Category::Ssh, "x", weight).with(status, "")
    }

    #[test]
    fn all_passed_is_grade_a() {
        let checks = vec![
            check(CheckStatus::Pass, Weight::High),
            check(CheckStatus::Pass, Weight::Low),
            check(CheckStatus::Unknown, Weight::High),
        ];
        let score = score(&checks);
        assert_eq!(score.grade, Grade::A);
        assert_eq!(score.percent, 100);
        assert_eq!(score.known, 2);
    }

    #[test]
    fn high_weight_failure_caps_grade_at_c() {
        let mut checks = vec![check(CheckStatus::Fail, Weight::High)];
        checks.extend((0..10).map(|_| check(CheckStatus::Pass, Weight::High)));
        let score = score(&checks);
        assert_eq!(score.percent, 90);
        assert_eq!(score.grade, Grade::C);
    }

    #[test]
    fn warnings_count_half() {
        let checks = vec![
            check(CheckStatus::Warn, Weight::Medium),
            check(CheckStatus::Pass, Weight::Medium),
        ];
        assert_eq!(score(&checks).percent, 75);
    }
}
