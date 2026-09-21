use std::collections::HashMap;

use super::model::{Deploy, DeployError, DeployStatus, Source, Stage, StageStatus};
use crate::common::systemd_time::systemd_time;

const UNIT_MARKER: &str = "@@ ";
const LOG_LINES: u32 = 60;
const ERROR_MARKERS: [&str; 5] = ["error", "Error", "ERROR", "failed", "Failed"];

pub const UNITS_SCRIPT: &str = "TZ=UTC systemctl show 'deploy*' -p Id -p ActiveState -p SubState -p Result -p ExecMainStartTimestamp -p ExecMainExitTimestamp -p ExecMainStatus --no-pager";

pub fn logs_script() -> String {
    format!(
        "for u in $(systemctl list-units 'deploy*' --all --plain --no-legend 2>/dev/null | awk '{{print $1}}'); do echo \"{UNIT_MARKER}$u\"; journalctl -u \"$u\" -n {LOG_LINES} -o short-iso --no-pager -q 2>/dev/null; done"
    )
}

pub fn deploys(units_raw: &str, logs_raw: &str) -> Vec<Deploy> {
    let logs = unit_logs(logs_raw);
    units_raw
        .split("\n\n")
        .filter_map(|block| unit_deploy(block, &logs))
        .collect()
}

fn unit_logs(raw: &str) -> HashMap<String, Vec<String>> {
    let mut logs = HashMap::new();
    for block in raw.split(UNIT_MARKER).skip(1) {
        let Some((unit, body)) = block.split_once('\n') else {
            continue;
        };
        let lines = body.lines().map(str::to_owned).collect();
        logs.insert(unit.trim().to_owned(), lines);
    }
    logs
}

fn unit_deploy(block: &str, logs: &HashMap<String, Vec<String>>) -> Option<Deploy> {
    let fields: HashMap<&str, &str> = block.lines().filter_map(|l| l.split_once('=')).collect();
    let unit = *fields.get("Id")?;
    let started = systemd_time(
        fields
            .get("ExecMainStartTimestamp")
            .copied()
            .unwrap_or_default(),
    );
    let exited = systemd_time(
        fields
            .get("ExecMainExitTimestamp")
            .copied()
            .unwrap_or_default(),
    );
    let active = fields.get("ActiveState").copied().unwrap_or_default();
    let result = fields.get("Result").copied().unwrap_or_default();
    let exit_status = fields.get("ExecMainStatus").copied().unwrap_or("0");
    let started_at = started?;
    let log_tail = logs.get(unit).cloned().unwrap_or_default();
    let (status, finished_at) = match (active, result) {
        ("activating" | "active" | "reloading" | "deactivating", _) => {
            (DeployStatus::InProgress, None)
        }
        ("failed", _) | (_, "exit-code" | "signal" | "core-dump" | "timeout") => {
            (DeployStatus::Failed, exited)
        }
        _ => (DeployStatus::Success, exited),
    };
    let mut stages = vec![Stage::done("start", Some(started_at))];
    stages.push(match status {
        DeployStatus::InProgress => Stage {
            name: "run".to_owned(),
            at: None,
            status: StageStatus::Active,
        },
        DeployStatus::Success => Stage::done("exit 0", exited),
        DeployStatus::Failed => Stage {
            name: format!("exit {exit_status}"),
            at: exited,
            status: StageStatus::Failed,
        },
    });
    Some(Deploy {
        key: format!("systemd:{unit}:{}", started_at.timestamp()),
        project: unit.trim_end_matches(".service").to_owned(),
        source: Source::Systemd,
        detail: format!("{unit} {active}/{result}"),
        started_at,
        finished_at,
        stages,
        status,
        error: (status == DeployStatus::Failed).then(|| error_from_log(&log_tail)),
        log_tail,
    })
}

fn error_from_log(lines: &[String]) -> DeployError {
    let index = lines
        .iter()
        .rposition(|l| ERROR_MARKERS.iter().any(|m| l.contains(m)))
        .unwrap_or(lines.len().saturating_sub(1));
    let line = lines.get(index).cloned().unwrap_or_default();
    let start = index.saturating_sub(super::model::ERROR_CONTEXT_LINES);
    DeployError {
        line,
        context: lines[start..].to_vec(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fixture_units_give_failed_and_finished_deploys() {
        let units = include_str!("../../fixtures/deploy/units.txt");
        let logs = include_str!("../../fixtures/deploy/unit_logs.txt");
        let deploys = deploys(units, logs);
        assert_eq!(deploys.len(), 2);
        let failed = deploys
            .iter()
            .find(|d| d.project == "deploy-api")
            .expect("failed unit");
        assert_eq!(failed.status, DeployStatus::Failed);
        assert!(failed.error.as_ref().expect("error").line.contains("ERROR"));
        assert_eq!(failed.stages[1].name, "exit 1");
        let ok = deploys
            .iter()
            .find(|d| d.project == "deploy-shop")
            .expect("ok unit");
        assert_eq!(ok.status, DeployStatus::Success);
        assert!(ok.finished_at.is_some());
    }
}
