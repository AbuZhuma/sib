use super::model::{Runner, RunnerKind};

pub const SCRIPT: &str = "for p in $(pgrep -f '[R]unner.Listener|[g]itlab-runner run'); do printf '%s\\t%s\\t%s\\n' \"$p\" \"$(readlink /proc/$p/cwd)\" \"$(tr '\\0' ' ' < /proc/$p/cmdline)\"; done";
pub const BUSY_SCRIPT: &str = "pgrep -c -f '[R]unner.Worker|[g]itlab-runner-helper'";

pub fn log_script(working_dir: &str) -> String {
    let quoted = asiba_core::transport::shell_quote(working_dir);
    format!(
        "f=$(ls -t {quoted}/_diag/Worker_*.log 2>/dev/null | head -1); [ -n \"$f\" ] && tail -n 40 \"$f\""
    )
}

pub fn runner(raw: &str, busy_raw: &str, log_raw: &str) -> Option<Runner> {
    let line = raw.lines().find(|l| !l.trim().is_empty())?;
    let fields: Vec<&str> = line.split('\t').collect();
    let working_dir = fields.get(1).copied().unwrap_or_default().to_owned();
    let command = fields.get(2).copied().unwrap_or_default();
    let kind = if command.contains("gitlab-runner") {
        RunnerKind::GitlabRunner
    } else {
        RunnerKind::GithubActions
    };
    let busy: u32 = busy_raw.trim().parse().unwrap_or(0);
    Some(Runner {
        kind,
        working_dir,
        is_busy: busy > 0,
        log_tail: log_raw.lines().map(str::to_owned).collect(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn github_runner_process_is_detected_as_busy() {
        let raw = "4242\t/opt/actions-runner\t/opt/actions-runner/bin/Runner.Listener run\n";
        let runner = runner(raw, "1\n", "line1\nline2\n").expect("runner");
        assert_eq!(runner.kind, RunnerKind::GithubActions);
        assert_eq!(runner.working_dir, "/opt/actions-runner");
        assert!(runner.is_busy);
        assert_eq!(runner.log_tail.len(), 2);
    }

    #[test]
    fn no_process_means_no_runner() {
        assert_eq!(runner("", "0", ""), None);
    }
}
