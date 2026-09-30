use asiba_core::{CheckKind, CustomCheck, ServerState, Severity};
use asiba_modules::checks::{self, CheckOutcome, CheckResult, ChecksSnapshot};

use super::check::AuditCheck;
use super::evidence::{Evidence, TAB_CHECKS};
use super::pattern::{Area, Outcome, Weight};

const PASSED: &str = "проверка пройдена";
const SOURCE_LIMIT: usize = 200;

pub fn checks(server: &ServerState) -> Vec<AuditCheck> {
    server
        .data::<ChecksSnapshot>(checks::ID)
        .map(|snapshot| snapshot.results.iter().map(to_check).collect())
        .unwrap_or_default()
}

fn to_check(result: &CheckResult) -> AuditCheck {
    AuditCheck {
        id: result.check.id.clone(),
        instance: None,
        area: Area::Custom,
        subject: result.check.name.clone(),
        description: description(&result.check),
        weight: weight(result.check.severity),
        outcome: outcome(result.outcome),
        detail: detail(result),
        advice: result.check.advice.clone(),
        evidence: Evidence::Tab(TAB_CHECKS),
    }
}

fn description(check: &CustomCheck) -> String {
    if !check.description.trim().is_empty() {
        return check.description.clone();
    }
    let source: String = check.source.trim().chars().take(SOURCE_LIMIT).collect();
    let prefix = match check.kind {
        CheckKind::Script => "Своя проверка, команда:",
        CheckKind::LocalFile => "Своя проверка, скрипт с этой машины:",
        CheckKind::RemoteFile => "Своя проверка, скрипт на сервере:",
    };
    format!("{prefix} {source}")
}

fn weight(severity: Severity) -> Weight {
    match severity {
        Severity::Critical => Weight::High,
        Severity::Warning => Weight::Medium,
        Severity::Info => Weight::Low,
    }
}

fn outcome(outcome: CheckOutcome) -> Outcome {
    match outcome {
        CheckOutcome::Passed => Outcome::Pass,
        CheckOutcome::Failed => Outcome::Fail,
        CheckOutcome::Skipped => Outcome::Skipped,
    }
}

fn detail(result: &CheckResult) -> String {
    match result.outcome {
        CheckOutcome::Passed => PASSED.to_owned(),
        CheckOutcome::Skipped => result.output.clone(),
        CheckOutcome::Failed if result.first_line().is_empty() => {
            format!("код возврата {}", result.exit_code)
        }
        CheckOutcome::Failed => result.first_line().to_owned(),
    }
}

#[cfg(test)]
mod tests {
    use asiba_core::{
        AuthMethod, Availability, ModuleState, ServerId, ServerSpec, Snapshot, SudoMode,
    };
    use chrono::Utc;

    use super::*;

    fn server(results: Vec<CheckResult>) -> ServerState {
        let mut server = bare();
        let mut module = ModuleState::detected(Availability::Available);
        module.record_snapshot(Snapshot::new(ChecksSnapshot { results }));
        server.modules.insert(checks::ID, module);
        server
    }

    fn bare() -> ServerState {
        ServerState::new(ServerSpec {
            id: ServerId::parse("neo").expect("id"),
            host: "h".into(),
            port: 22,
            user: "root".into(),
            auth: AuthMethod::Auto,
            jump: None,
            sudo: SudoMode::None,
            description: Default::default(),
            location: None,
            modules: Default::default(),
        })
    }

    fn result(outcome: CheckOutcome, output: &str) -> CheckResult {
        CheckResult {
            check: CustomCheck {
                name: "Сертификат".to_owned(),
                severity: Severity::Critical,
                advice: "продлить".to_owned(),
                source: "openssl x509 -checkend 604800".to_owned(),
                ..CustomCheck::new("custom:1")
            },
            outcome,
            exit_code: 1,
            output: output.to_owned(),
            at: Utc::now(),
        }
    }

    #[test]
    fn failed_check_becomes_a_high_weight_failure() {
        let audit = checks(&server(vec![result(CheckOutcome::Failed, "истекает")]));
        assert_eq!(audit.len(), 1);
        assert_eq!(audit[0].outcome, Outcome::Fail);
        assert_eq!(audit[0].weight, Weight::High);
        assert_eq!(audit[0].detail, "истекает");
        assert_eq!(audit[0].area, Area::Custom);
    }

    #[test]
    fn failed_check_without_output_shows_the_exit_code() {
        let audit = checks(&server(vec![result(CheckOutcome::Failed, "")]));
        assert_eq!(audit[0].detail, "код возврата 1");
    }

    #[test]
    fn description_falls_back_to_the_command() {
        let audit = checks(&server(vec![result(CheckOutcome::Passed, "")]));
        assert!(audit[0].description.contains("openssl x509"));
        assert_eq!(audit[0].outcome, Outcome::Pass);
    }

    #[test]
    fn server_without_the_module_has_no_custom_checks() {
        assert!(checks(&bare()).is_empty());
    }
}
