use std::collections::BTreeMap;

use chrono::{DateTime, Duration, Utc};

use super::model::{Deploy, DeployError, DeployStatus, Source, Stage, StageStatus};

const DEPLOY_GAP: Duration = Duration::seconds(120);
const SETTLE: Duration = Duration::seconds(30);
const EVENTS_KEEP: Duration = Duration::hours(24);
const MAX_EVENTS: usize = 4000;
const DOCKER_FORMAT: &str = "{{.Time}}\\t{{.Type}}\\t{{.Action}}\\t{{.Actor.Attributes.name}}\\t{{index .Actor.Attributes \"com.docker.compose.project\"}}\\t{{index .Actor.Attributes \"com.docker.compose.service\"}}\\t{{.Actor.Attributes.image}}\\t{{.Actor.Attributes.exitCode}}";
const PODMAN_FORMAT: &str = "{{.Time}}\\t{{.Type}}\\t{{.Status}}\\t{{.Name}}\\t{{index .Attributes \"com.docker.compose.project\"}}\\t{{index .Attributes \"com.docker.compose.service\"}}\\t{{.Image}}\\t{{.ContainerExitCode}}";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ComposeEvent {
    pub at: DateTime<Utc>,
    pub kind: String,
    pub action: String,
    pub name: String,
    pub project: String,
    pub exit_code: Option<i32>,
}

pub fn script(since: i64, until: i64) -> String {
    format!(
        "D=$(command -v docker || command -v podman); case \"$D\" in *podman*) F='{PODMAN_FORMAT}';; *) F='{DOCKER_FORMAT}';; esac; [ -n \"$D\" ] && $D events --since {since} --until {until} --format \"$F\""
    )
}

pub fn events(raw: &str) -> Vec<ComposeEvent> {
    raw.lines().filter_map(event_line).collect()
}

fn event_line(line: &str) -> Option<ComposeEvent> {
    let fields: Vec<&str> = line.split('\t').collect();
    if fields.len() < 7 {
        return None;
    }
    let at = DateTime::from_timestamp(fields[0].trim().parse().ok()?, 0)?;
    let action = fields[2].trim();
    if !is_relevant(fields[1], action) {
        return None;
    }
    let project = if fields[4].is_empty() {
        fields[3].to_owned()
    } else {
        fields[4].to_owned()
    };
    Some(ComposeEvent {
        at,
        kind: fields[1].to_owned(),
        action: action.to_owned(),
        name: fields[3].to_owned(),
        project,
        exit_code: fields.get(7).and_then(|c| c.trim().parse().ok()),
    })
}

fn is_relevant(kind: &str, action: &str) -> bool {
    match kind {
        "image" => action == "pull",
        "container" => {
            matches!(action, "create" | "start" | "restart" | "die" | "destroy")
                || action.starts_with("health_status")
        }
        _ => false,
    }
}

pub fn merge_events(
    previous: &[ComposeEvent],
    fresh: Vec<ComposeEvent>,
    now: DateTime<Utc>,
) -> Vec<ComposeEvent> {
    let cutoff = now - EVENTS_KEEP;
    let mut merged: Vec<ComposeEvent> = previous
        .iter()
        .filter(|e| e.at >= cutoff)
        .cloned()
        .collect();
    for event in fresh {
        if event.at >= cutoff && !merged.contains(&event) {
            merged.push(event);
        }
    }
    merged.sort_by_key(|e| e.at);
    if merged.len() > MAX_EVENTS {
        let overflow = merged.len() - MAX_EVENTS;
        merged.drain(..overflow);
    }
    merged
}

pub fn deploys(events: &[ComposeEvent], now: DateTime<Utc>) -> Vec<Deploy> {
    let mut by_project: BTreeMap<&str, Vec<&ComposeEvent>> = BTreeMap::new();
    for event in events {
        by_project.entry(&event.project).or_default().push(event);
    }
    let mut deploys = Vec::new();
    for (project, events) in by_project {
        for window in windows(&events) {
            if let Some(deploy) = build(project, &window, now) {
                deploys.push(deploy);
            }
        }
    }
    deploys
}

fn windows<'a>(events: &[&'a ComposeEvent]) -> Vec<Vec<&'a ComposeEvent>> {
    let mut windows: Vec<Vec<&ComposeEvent>> = Vec::new();
    for event in events {
        match windows.last_mut() {
            Some(window) if window.last().is_some_and(|l| event.at - l.at <= DEPLOY_GAP) => {
                window.push(event);
            }
            _ => windows.push(vec![event]),
        }
    }
    windows
}

fn build(project: &str, window: &[&ComposeEvent], now: DateTime<Utc>) -> Option<Deploy> {
    let first = window.first()?;
    let last = window.last()?;
    if !window
        .iter()
        .any(|e| matches!(e.action.as_str(), "create" | "start" | "restart"))
    {
        return None;
    }
    let mut stages = stages_of(window);
    let crash = crash_of(window);
    let (status, finished_at) = window_status(crash.is_some(), last.at, now);
    if let Some(c) = crash {
        stages.push(Stage {
            name: format!("{} exited", c.name),
            at: Some(c.at),
            status: StageStatus::Failed,
        });
    }
    Some(Deploy {
        key: format!("compose:{project}:{}", first.at.timestamp()),
        project: project.to_owned(),
        source: Source::Compose,
        detail: container_names(window).join(", "),
        started_at: first.at,
        finished_at,
        stages,
        status,
        error: crash.map(|c| DeployError {
            line: format!("{} exited with code {}", c.name, c.exit_code.unwrap_or(0)),
            context: Vec::new(),
        }),
        log_tail: Vec::new(),
    })
}

fn window_status(
    failed: bool,
    last_at: DateTime<Utc>,
    now: DateTime<Utc>,
) -> (DeployStatus, Option<DateTime<Utc>>) {
    if failed {
        (DeployStatus::Failed, Some(last_at))
    } else if now - last_at < SETTLE {
        (DeployStatus::InProgress, None)
    } else {
        (DeployStatus::Success, Some(last_at))
    }
}

fn stages_of(window: &[&ComposeEvent]) -> Vec<Stage> {
    let mut stages = Vec::new();
    for (action, name) in [("pull", "pull"), ("create", "create"), ("start", "start")] {
        if let Some(event) = window.iter().find(|e| e.action == action) {
            stages.push(Stage::done(name, Some(event.at)));
        }
    }
    if let Some(event) = window.iter().find(|e| e.action.contains("healthy")) {
        stages.push(Stage::done("healthy", Some(event.at)));
    }
    stages
}

fn crash_of<'a>(window: &[&'a ComposeEvent]) -> Option<&'a ComposeEvent> {
    let crash = window
        .iter()
        .rev()
        .find(|e| e.action == "die" && e.exit_code.is_some_and(|c| c != 0))?;
    let last_start = window.iter().rev().find(|e| e.action == "start");
    last_start.is_none_or(|s| crash.at >= s.at).then_some(crash)
}

fn container_names<'a>(window: &[&'a ComposeEvent]) -> Vec<&'a str> {
    let mut names: Vec<&str> = window
        .iter()
        .map(|e| e.name.as_str())
        .filter(|n| !n.is_empty())
        .collect();
    names.sort_unstable();
    names.dedup();
    names
}

#[cfg(test)]
mod tests {
    use super::*;

    fn now() -> DateTime<Utc> {
        DateTime::from_timestamp(1_757_900_000, 0).expect("now")
    }

    #[test]
    fn fixture_events_form_deploys_per_project() {
        let raw = include_str!("../../fixtures/deploy/events.txt");
        let events = events(raw);
        let deploys = deploys(&events, now());
        assert_eq!(deploys.len(), 2);
        let shop = deploys.iter().find(|d| d.project == "shop").expect("shop");
        assert_eq!(shop.status, DeployStatus::Success);
        let names: Vec<&str> = shop.stages.iter().map(|s| s.name.as_str()).collect();
        assert_eq!(names, vec!["pull", "create", "start", "healthy"]);
        let api = deploys.iter().find(|d| d.project == "api").expect("api");
        assert_eq!(api.status, DeployStatus::Failed);
        assert!(api.error.as_ref().expect("error").line.contains("code 1"));
    }

    #[test]
    fn exec_events_are_ignored() {
        assert!(events("1757899000\tcontainer\texec\tdb\t\t\timg\t\n").is_empty());
    }

    #[test]
    fn merge_events_dedupes_and_sorts() {
        let raw = include_str!("../../fixtures/deploy/events.txt");
        let fresh = events(raw);
        let merged = merge_events(&fresh[..3], fresh.clone(), now());
        assert_eq!(merged.len(), fresh.len());
        assert!(merged.windows(2).all(|w| w[0].at <= w[1].at));
    }
}
