use super::check::AuditCheck;
use super::pattern::{Outcome, Weight};

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
pub struct SecurityScore {
    pub grade: Grade,
    pub percent: u32,
    pub passed: usize,
    pub known: usize,
    pub failed_high: usize,
}

pub fn security_score<'a>(checks: impl Iterator<Item = &'a AuditCheck>) -> SecurityScore {
    let known: Vec<&AuditCheck> = checks
        .filter(|c| c.area.is_security() && c.outcome != Outcome::Skipped)
        .collect();
    let total: u32 = known.iter().map(|c| c.weight.points()).sum();
    let earned: u32 = known
        .iter()
        .map(|c| match c.outcome {
            Outcome::Pass => c.weight.points(),
            Outcome::Warn => c.weight.points() / 2,
            _ => 0,
        })
        .sum();
    let percent = (earned * 100).checked_div(total).unwrap_or(0);
    let failed_high = known
        .iter()
        .filter(|c| c.outcome == Outcome::Fail && c.weight == Weight::High)
        .count();
    let grade = match (percent, failed_high) {
        (90.., 0) => Grade::A,
        (75.., 0) => Grade::B,
        (60.., _) => Grade::C,
        (40.., _) => Grade::D,
        _ => Grade::F,
    };
    SecurityScore {
        grade,
        percent,
        passed: known.iter().filter(|c| c.outcome == Outcome::Pass).count(),
        known: known.len(),
        failed_high,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::audit::evidence::Evidence;
    use crate::audit::pattern::Area;

    fn check(outcome: Outcome, weight: Weight) -> AuditCheck {
        AuditCheck {
            id: "t",
            instance: None,
            area: Area::Ssh,
            subject: "x",
            description: "",
            weight,
            outcome,
            detail: String::new(),
            advice: "",
            evidence: Evidence::None,
        }
    }

    #[test]
    fn all_passed_is_grade_a_and_skipped_are_not_counted() {
        let checks = [
            check(Outcome::Pass, Weight::High),
            check(Outcome::Pass, Weight::Low),
            check(Outcome::Skipped, Weight::High),
        ];
        let score = security_score(checks.iter());
        assert_eq!(score.grade, Grade::A);
        assert_eq!(score.percent, 100);
        assert_eq!(score.known, 2);
    }

    #[test]
    fn high_weight_failure_caps_grade_at_c() {
        let mut checks = vec![check(Outcome::Fail, Weight::High)];
        checks.extend((0..10).map(|_| check(Outcome::Pass, Weight::High)));
        let score = security_score(checks.iter());
        assert_eq!(score.percent, 90);
        assert_eq!(score.grade, Grade::C);
    }

    #[test]
    fn warnings_count_half_and_non_security_areas_are_ignored() {
        let mut checks = vec![
            check(Outcome::Warn, Weight::Medium),
            check(Outcome::Pass, Weight::Medium),
        ];
        let mut other = check(Outcome::Fail, Weight::High);
        other.area = Area::Resources;
        checks.push(other);
        assert_eq!(security_score(checks.iter()).percent, 75);
    }
}
