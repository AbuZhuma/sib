use sib_core::{Event, Severity};

use super::model::{CheckOutcome, CheckResult, ChecksSnapshot};
use super::{ID, model};

pub fn changes(previous: Option<&ChecksSnapshot>, current: &[CheckResult]) -> Vec<Event> {
    let Some(previous) = previous else {
        return Vec::new();
    };
    current
        .iter()
        .filter_map(|result| change(previous, result))
        .collect()
}

fn change(previous: &ChecksSnapshot, result: &CheckResult) -> Option<Event> {
    let before = previous.find(&result.check.id)?.outcome;
    if before == result.outcome {
        return None;
    }
    match result.outcome {
        CheckOutcome::Failed => Some(Event::new(
            ID,
            result.check.weight.severity(),
            failed_message(result),
        )),
        CheckOutcome::Passed if before == CheckOutcome::Failed => Some(Event::new(
            ID,
            Severity::Info,
            format!("check \"{}\" passes again", result.check.name),
        )),
        _ => None,
    }
}

fn failed_message(result: &CheckResult) -> String {
    let detail = model::CheckResult::first_line(result);
    if detail.is_empty() {
        return format!(
            "check \"{}\" failed (exit code {})",
            result.check.name, result.exit_code
        );
    }
    format!("check \"{}\" failed: {detail}", result.check.name)
}

#[cfg(test)]
mod tests {
    use chrono::Utc;
    use sib_core::CustomCheck;

    use super::*;

    fn result(outcome: CheckOutcome) -> CheckResult {
        CheckResult {
            check: CustomCheck {
                name: "Certificate".to_owned(),
                ..CustomCheck::new("custom:1")
            },
            outcome,
            exit_code: 1,
            output: "expires in 2 days".to_owned(),
            at: Utc::now(),
        }
    }

    #[test]
    fn first_failure_after_a_pass_makes_an_event() {
        let previous = ChecksSnapshot {
            results: vec![result(CheckOutcome::Passed)],
        };
        let events = changes(Some(&previous), &[result(CheckOutcome::Failed)]);
        assert_eq!(events.len(), 1);
        assert!(events[0].message.contains("expires in 2 days"));
    }

    #[test]
    fn unchanged_outcome_makes_no_event() {
        let previous = ChecksSnapshot {
            results: vec![result(CheckOutcome::Failed)],
        };
        assert!(changes(Some(&previous), &[result(CheckOutcome::Failed)]).is_empty());
    }

    #[test]
    fn recovery_makes_an_info_event() {
        let previous = ChecksSnapshot {
            results: vec![result(CheckOutcome::Failed)],
        };
        let events = changes(Some(&previous), &[result(CheckOutcome::Passed)]);
        assert_eq!(events.first().map(|e| e.severity), Some(Severity::Info));
    }

    #[test]
    fn first_collect_makes_no_events() {
        assert!(changes(None, &[result(CheckOutcome::Failed)]).is_empty());
    }
}
