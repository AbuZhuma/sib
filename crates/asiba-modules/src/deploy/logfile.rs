use std::sync::LazyLock;

use chrono::{DateTime, Duration, Utc};
use regex::Regex;

use super::model::{
    Deploy, DeployError, DeployStatus, ERROR_CONTEXT_LINES, Source, Stage, StageStatus,
};

pub const SETTING_LOGS: &str = "logs";
const TAIL_LINES: u32 = 400;
const TAIL_KEPT: usize = 40;
const FILE_MARKER: &str = "@@ ";
const ACTIVE_WINDOW: Duration = Duration::seconds(120);

static STAGE_PATTERNS: LazyLock<Vec<(Regex, &'static str)>> = LazyLock::new(|| {
    [
        (r"(?i)^\s*(git pull|Updating [0-9a-f]+\.\.[0-9a-f]+|Fast-forward)", "fetch"),
        (r"(?i)\b(Pulling|Pulled)\b", "pull"),
        (r"(?i)\b(npm (ci|install)|pip install|Collecting |added \d+ packages)", "install"),
        (r"(?i)\b(Compiling|npm run build|> .*\bbuild\b|Building|docker build)", "build"),
        (r"(?i)\b(Creating|Created|Recreat)", "create"),
        (r"(?i)\b(Starting|Started|systemctl restart|Restarting)", "start"),
        (r"(?i)\b(migrat)", "migrate"),
        (r"(?i)\b(Healthy|health check passed)", "healthy"),
        (
            r"(?i)(deploy(ed|ment)? (complete|finished|done|successful)|Successfully installed|Finished .*(release|dev)|^\s*done\s*$)",
            "done",
        ),
    ]
    .into_iter()
    .filter_map(|(pattern, name)| Regex::new(pattern).ok().map(|r| (r, name)))
    .collect()
});

static ERROR_PATTERN: LazyLock<Option<Regex>> = LazyLock::new(|| {
    Regex::new(r"(?i)(^\s*error\b|error\[|npm ERR!|ERROR:|Traceback|\bfailed\b|FAILED|fatal:|panicked|exit code [1-9]|exited with|Cannot |Could not )").ok()
});

pub fn script(paths: &[String]) -> String {
    paths
        .iter()
        .map(|path| {
            let quoted = asiba_core::transport::shell_quote(path);
            format!(
                "[ -r {quoted} ] && {{ echo \"{FILE_MARKER}{path} $(stat -c %Y {quoted})\"; tail -n {TAIL_LINES} {quoted}; }}"
            )
        })
        .collect::<Vec<_>>()
        .join("; ")
}

pub fn configured_paths(setting: Option<&String>) -> Vec<String> {
    setting
        .map(|s| {
            s.split(',')
                .map(str::trim)
                .filter(|p| !p.is_empty())
                .map(str::to_owned)
                .collect()
        })
        .unwrap_or_default()
}

pub fn deploys(raw: &str, previous: &[Deploy], now: DateTime<Utc>) -> Vec<Deploy> {
    raw.split(FILE_MARKER)
        .skip(1)
        .filter_map(|block| file_deploy(block, previous, now))
        .collect()
}

fn file_deploy(block: &str, previous: &[Deploy], now: DateTime<Utc>) -> Option<Deploy> {
    let (header, body) = block.split_once('\n').unwrap_or((block, ""));
    let (path, mtime) = header.trim().rsplit_once(' ')?;
    let modified = DateTime::from_timestamp(mtime.parse().ok()?, 0)?;
    let lines: Vec<&str> = body.lines().collect();
    let key = format!("log:{path}");
    let analysis = analyze(&lines);
    let status = file_status(&analysis, now - modified < ACTIVE_WINDOW);
    let earlier = previous.iter().find(|d| d.key == key);
    let started_at = match earlier {
        Some(d) if d.status == DeployStatus::InProgress => d.started_at,
        _ => modified,
    };
    let mut stages = analysis.stages;
    if status == DeployStatus::InProgress
        && let Some(last) = stages.last_mut()
    {
        last.status = StageStatus::Active;
    }
    Some(Deploy {
        key,
        project: project_name(path.trim_end_matches(".log")),
        source: Source::LogFile,
        detail: path.to_owned(),
        started_at,
        finished_at: (status != DeployStatus::InProgress).then_some(modified),
        stages,
        status,
        error: analysis.error_index.map(|i| error_at(&lines, i)),
        log_tail: tail(&lines, TAIL_KEPT),
    })
}

fn file_status(analysis: &Analysis, is_active: bool) -> DeployStatus {
    if analysis.error_index.is_some() {
        DeployStatus::Failed
    } else if analysis.is_finished || !is_active {
        DeployStatus::Success
    } else {
        DeployStatus::InProgress
    }
}

fn tail(lines: &[&str], count: usize) -> Vec<String> {
    let start = lines.len().saturating_sub(count);
    lines[start..].iter().map(|l| (*l).to_owned()).collect()
}

struct Analysis {
    stages: Vec<Stage>,
    error_index: Option<usize>,
    is_finished: bool,
}

fn analyze(lines: &[&str]) -> Analysis {
    let mut stages: Vec<Stage> = Vec::new();
    let mut last_stage_line = 0;
    for (index, line) in lines.iter().enumerate() {
        let Some((_, name)) = STAGE_PATTERNS.iter().find(|(r, _)| r.is_match(line)) else {
            continue;
        };
        if stages.last().is_none_or(|s| s.name != *name) {
            stages.push(Stage::done(*name, None));
        }
        last_stage_line = index;
    }
    let error_index = ERROR_PATTERN.as_ref().and_then(|r| {
        lines
            .iter()
            .enumerate()
            .skip(last_stage_line.saturating_sub(ERROR_CONTEXT_LINES))
            .rev()
            .find(|(_, l)| r.is_match(l))
            .map(|(i, _)| i)
    });
    let is_finished = stages
        .last()
        .is_some_and(|s| s.name == "done" || s.name == "healthy");
    if let (Some(index), Some(stage)) = (error_index, stages.last_mut())
        && index >= last_stage_line
    {
        stage.status = StageStatus::Failed;
    }
    Analysis {
        stages,
        error_index: error_index.filter(|i| *i >= last_stage_line || !is_finished),
        is_finished,
    }
}

fn error_at(lines: &[&str], index: usize) -> DeployError {
    let start = index.saturating_sub(ERROR_CONTEXT_LINES);
    let end = (index + ERROR_CONTEXT_LINES + 1).min(lines.len());
    DeployError {
        line: lines[index].to_owned(),
        context: lines[start..end].iter().map(|l| (*l).to_owned()).collect(),
    }
}

fn project_name(path: &str) -> String {
    path.trim_end_matches('/')
        .rsplit('/')
        .next()
        .unwrap_or(path)
        .to_owned()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn now() -> DateTime<Utc> {
        DateTime::from_timestamp(1_757_900_000, 0).expect("now")
    }

    #[test]
    fn compose_log_with_error_is_failed_at_start_stage() {
        let raw = include_str!("../../fixtures/deploy/compose_up.log");
        let deploys = deploys(raw, &[], now());
        assert_eq!(deploys.len(), 1);
        let deploy = &deploys[0];
        assert_eq!(deploy.project, "shop-deploy");
        assert_eq!(deploy.status, DeployStatus::Failed);
        let names: Vec<&str> = deploy.stages.iter().map(|s| s.name.as_str()).collect();
        assert_eq!(names, vec!["fetch", "pull", "build", "create", "start"]);
        assert!(
            deploy
                .error
                .as_ref()
                .expect("error")
                .line
                .contains("exited with")
        );
        assert_eq!(
            deploy.stages.last().map(|s| s.status),
            Some(StageStatus::Failed)
        );
    }

    #[test]
    fn cargo_log_finished_is_success() {
        let raw = format!(
            "{FILE_MARKER}/srv/app/deploy.log 1757899000\n   Compiling app v0.1.0\n    Finished release [optimized] target(s) in 30s\n"
        );
        let deploys = deploys(&raw, &[], now());
        assert_eq!(deploys[0].status, DeployStatus::Success);
        assert!(deploys[0].error.is_none());
    }

    #[test]
    fn fresh_log_without_finish_is_in_progress_and_keeps_start() {
        let earlier = Deploy {
            key: "log:/srv/app/deploy.log".to_owned(),
            project: "app".to_owned(),
            source: Source::LogFile,
            detail: String::new(),
            started_at: DateTime::from_timestamp(1_757_899_000, 0).expect("t"),
            finished_at: None,
            stages: Vec::new(),
            status: DeployStatus::InProgress,
            error: None,
            log_tail: Vec::new(),
        };
        let raw = format!("{FILE_MARKER}/srv/app/deploy.log 1757899990\nPulling web\n");
        let deploys = deploys(&raw, std::slice::from_ref(&earlier), now());
        assert_eq!(deploys[0].status, DeployStatus::InProgress);
        assert_eq!(deploys[0].started_at, earlier.started_at);
        assert_eq!(deploys[0].stages[0].status, StageStatus::Active);
    }
}
