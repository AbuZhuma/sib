use asiba_core::{CustomCheck, OUTPUT_LIMIT};
use chrono::Utc;

use super::model::{CheckOutcome, CheckResult};
use super::script::{EXIT_MARKER, START_MARKER};

pub fn results(raw: &str, checks: &[&CustomCheck]) -> Vec<CheckResult> {
    let mut results = Vec::new();
    let mut current: Option<(usize, Vec<&str>)> = None;
    for line in raw.lines() {
        if let Some(index) = line.strip_prefix(START_MARKER) {
            current = index.trim().parse().ok().map(|index| (index, Vec::new()));
            continue;
        }
        if let Some(code) = line.strip_prefix(EXIT_MARKER) {
            let Some((index, lines)) = current.take() else {
                continue;
            };
            let Some(check) = checks.get(index) else {
                continue;
            };
            results.push(result(check, code.trim().parse().unwrap_or(-1), &lines));
            continue;
        }
        if let Some((_, lines)) = current.as_mut() {
            lines.push(line);
        }
    }
    results
}

fn result(check: &CustomCheck, exit_code: i32, lines: &[&str]) -> CheckResult {
    let output = truncate(&lines.join("\n"));
    let is_passing = check.is_passing(exit_code, &output);
    CheckResult {
        check: (*check).clone(),
        outcome: if is_passing {
            CheckOutcome::Passed
        } else {
            CheckOutcome::Failed
        },
        exit_code,
        output,
        at: Utc::now(),
    }
}

fn truncate(output: &str) -> String {
    let trimmed = output.trim();
    match trimmed.char_indices().nth(OUTPUT_LIMIT) {
        Some((offset, _)) => trimmed[..offset].to_owned(),
        None => trimmed.to_owned(),
    }
}

#[cfg(test)]
mod tests {
    use asiba_core::CheckExpect;

    use super::*;

    fn check(id: &str, expect: CheckExpect, text: &str) -> CustomCheck {
        CustomCheck {
            name: id.to_owned(),
            source: "true".to_owned(),
            expect,
            expect_text: text.to_owned(),
            ..CustomCheck::new(id)
        }
    }

    #[test]
    fn exit_code_and_output_are_read_per_check() {
        let first = check("a", CheckExpect::ExitZero, "");
        let second = check("b", CheckExpect::ExitZero, "");
        let raw = "###asiba-check 0\nfine\n###asiba-exit 0\n###asiba-check 1\nboom\nagain\n###asiba-exit 3\n";
        let results = results(raw, &[&first, &second]);
        assert_eq!(results.len(), 2);
        assert_eq!(results[0].outcome, CheckOutcome::Passed);
        assert_eq!(results[0].output, "fine");
        assert_eq!(results[1].outcome, CheckOutcome::Failed);
        assert_eq!(results[1].exit_code, 3);
        assert_eq!(results[1].output, "boom\nagain");
    }

    #[test]
    fn contains_expectation_is_applied_to_the_output() {
        let check = check("a", CheckExpect::Contains, "running");
        let raw = "###asiba-check 0\nunit is running\n###asiba-exit 1\n";
        assert_eq!(results(raw, &[&check])[0].outcome, CheckOutcome::Passed);
    }

    #[test]
    fn output_without_exit_marker_is_dropped() {
        let check = check("a", CheckExpect::ExitZero, "");
        assert!(results("###asiba-check 0\nhalf", &[&check]).is_empty());
    }

    #[test]
    fn long_output_is_cut_to_the_limit() {
        let check = check("a", CheckExpect::ExitZero, "");
        let raw = format!("###asiba-check 0\n{}\n###asiba-exit 0\n", "x".repeat(9000));
        assert_eq!(
            results(&raw, &[&check])[0].output.chars().count(),
            OUTPUT_LIMIT
        );
    }
}
