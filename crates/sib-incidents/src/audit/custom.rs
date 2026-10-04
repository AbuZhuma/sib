use sib_core::{CheckKind, CustomCheck, ServerState};
use sib_modules::checks::{self, CheckOutcome, CheckResult, ChecksSnapshot};

use super::check::AuditCheck;
use super::evidence::{Evidence, TAB_CHECKS};
use super::pattern::Outcome;

const PASSED: &str = "check passed";
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
        area: result.check.area,
        subject: result.check.name.clone(),
        description: description(&result.check),
        weight: result.check.weight,
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
        CheckKind::Script => "Your own check, command:",
        CheckKind::LocalFile => "Your own check, script from this machine:",
        CheckKind::RemoteFile => "Your own check, script on the server:",
    };
    format!("{prefix} {source}")
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
            format!("exit code {}", result.exit_code)
        }
        CheckOutcome::Failed => result.first_line().to_owned(),
    }
}

#[cfg(test)]
mod tests {
    use chrono::Utc;
    use sib_core::{
        Area, AuthMethod, Availability, ModuleState, ServerId, ServerSpec, Snapshot, SudoMode,
        Weight,
    };

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
            checks: Vec::new(),
            check_overrides: Default::default(),
            pipelines: Vec::new(),
        })
    }

    fn result(outcome: CheckOutcome, output: &str) -> CheckResult {
        CheckResult {
            check: CustomCheck {
                name: "Certificate".to_owned(),
                weight: Weight::High,
                advice: "renew it".to_owned(),
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
        let audit = checks(&server(vec![result(CheckOutcome::Failed, "expiring")]));
        assert_eq!(audit.len(), 1);
        assert_eq!(audit[0].outcome, Outcome::Fail);
        assert_eq!(audit[0].weight, Weight::High);
        assert_eq!(audit[0].detail, "expiring");
        assert_eq!(audit[0].area, Area::Custom);
    }

    #[test]
    fn failed_check_without_output_shows_the_exit_code() {
        let audit = checks(&server(vec![result(CheckOutcome::Failed, "")]));
        assert_eq!(audit[0].detail, "exit code 1");
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
