mod compose;
mod logfile;
mod model;
mod runner;
mod systemd;

use async_trait::async_trait;
use chrono::{DateTime, Duration, Utc};
use sib_core::{
    Availability, CollectContext, Event, Module, ModuleError, ModuleId, ModuleSettings, Sample,
    Schedule, Severity, Snapshot, Transport,
};

pub use compose::ComposeEvent;
pub use logfile::SETTING_LOGS;
pub use model::{
    Deploy, DeployError, DeploySnapshot, DeployStatus, MAX_DEPLOYS, Runner, RunnerKind, Source,
    Stage, StageStatus,
};

use crate::common::sections::{self, Sections};

pub const ID: ModuleId = ModuleId("deploy");
pub const KEY_ACTIVE: &str = "deploy.active";
pub const KEY_FAILED: &str = "deploy.failed";

const INITIAL_EVENTS_WINDOW: Duration = Duration::hours(24);
const EVENTS_OVERLAP: Duration = Duration::seconds(5);

pub struct DeployModule;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DeployState {
    pub snapshot: DeploySnapshot,
    pub compose_events: Vec<ComposeEvent>,
}

fn detect_script(log_paths: &[String]) -> String {
    let parts = [
        ("docker", "command -v docker || command -v podman"),
        (
            "units",
            "systemctl list-units 'deploy*' --all --plain --no-legend | head -1",
        ),
        (
            "runner",
            "pgrep -f '[R]unner.Listener|[g]itlab-runner run' | head -1",
        ),
    ];
    let logs = log_paths
        .iter()
        .map(|p| {
            format!(
                "[ -r {} ] && echo {}",
                sib_core::transport::shell_quote(p),
                sib_core::transport::shell_quote(p)
            )
        })
        .collect::<Vec<_>>()
        .join("; ");
    format!("{}; echo '###logs'; {logs}", sections::script(&parts))
}

fn collect_script(
    since: i64,
    until: i64,
    log_paths: &[String],
    runner_dir: Option<&str>,
) -> String {
    let events = compose::script(since, until);
    let unit_logs = systemd::logs_script();
    let logs = logfile::script(log_paths);
    let runner_log = runner_dir.map(runner::log_script).unwrap_or_default();
    let parts = [
        ("events", events.as_str()),
        ("units", systemd::UNITS_SCRIPT),
        ("unitlogs", unit_logs.as_str()),
        ("logs", logs.as_str()),
        ("runner", runner::SCRIPT),
        ("runnerbusy", runner::BUSY_SCRIPT),
        ("runnerlog", runner_log.as_str()),
    ];
    sections::script(&parts)
}

#[async_trait]
impl Module for DeployModule {
    fn id(&self) -> ModuleId {
        ID
    }

    fn title(&self) -> &'static str {
        "Deploys"
    }

    fn schedule(&self) -> Schedule {
        Schedule::Normal
    }

    async fn detect(
        &self,
        transport: &dyn Transport,
        settings: &ModuleSettings,
    ) -> Result<Availability, ModuleError> {
        let log_paths = logfile::configured_paths(settings.get(SETTING_LOGS));
        let output = transport.exec(&detect_script(&log_paths)).await?;
        let sections = Sections::parse(&output.stdout);
        let has_source = ["docker", "units", "runner", "logs"]
            .iter()
            .any(|name| !sections.get_or_empty(name).trim().is_empty());
        if !has_source {
            return Ok(Availability::unavailable(
                "no deploy sources (docker, deploy*.service, CI runner)",
            ));
        }
        Ok(Availability::Available)
    }

    async fn collect(
        &self,
        transport: &dyn Transport,
        context: &CollectContext,
    ) -> Result<Snapshot, ModuleError> {
        let now = Utc::now();
        let previous = context.previous::<DeployState>().map(|(p, _)| p);
        let since = previous
            .map(|p| p.snapshot.events_since)
            .unwrap_or_else(|| (now - INITIAL_EVENTS_WINDOW).timestamp());
        let log_paths = logfile::configured_paths(context.settings.get(SETTING_LOGS));
        let runner_dir = previous
            .and_then(|p| p.snapshot.runner.as_ref())
            .map(|r| r.working_dir.as_str());
        let script = collect_script(since, now.timestamp(), &log_paths, runner_dir);
        let output = transport.exec(&script).await?;
        let state = build_state(&output.stdout, previous, now);
        let events = previous
            .map(|p| events_between(&p.snapshot, &state.snapshot))
            .unwrap_or_default();
        let samples = vec![
            Sample::new(KEY_ACTIVE, state.snapshot.active().count() as f64),
            Sample::new(KEY_FAILED, state.snapshot.failed_count() as f64),
        ];
        Ok(Snapshot::new(state)
            .with_samples(samples)
            .with_events(events))
    }
}

fn build_state(raw: &str, previous: Option<&DeployState>, now: DateTime<Utc>) -> DeployState {
    let sections = Sections::parse(raw);
    let previous_events = previous
        .map(|p| p.compose_events.as_slice())
        .unwrap_or_default();
    let previous_deploys = previous
        .map(|p| p.snapshot.deploys.as_slice())
        .unwrap_or_default();
    let compose_events = compose::merge_events(
        previous_events,
        compose::events(sections.get_or_empty("events")),
        now,
    );
    let mut fresh = compose::deploys(&compose_events, now);
    fresh.extend(systemd::deploys(
        sections.get_or_empty("units"),
        sections.get_or_empty("unitlogs"),
    ));
    fresh.extend(logfile::deploys(
        sections.get_or_empty("logs"),
        previous_deploys,
        now,
    ));
    let deploys = merge_deploys(previous_deploys, fresh);
    let runner = runner::runner(
        sections.get_or_empty("runner"),
        sections.get_or_empty("runnerbusy"),
        sections.get_or_empty("runnerlog"),
    );
    DeployState {
        snapshot: DeploySnapshot {
            deploys,
            runner,
            events_since: (now - EVENTS_OVERLAP).timestamp(),
        },
        compose_events,
    }
}

fn merge_deploys(previous: &[Deploy], fresh: Vec<Deploy>) -> Vec<Deploy> {
    let mut merged = fresh;
    for old in previous {
        if !merged.iter().any(|d| d.key == old.key) {
            merged.push(old.clone());
        }
    }
    merged.sort_by_key(|d| std::cmp::Reverse(d.started_at));
    merged.truncate(MAX_DEPLOYS);
    merged
}

fn events_between(previous: &DeploySnapshot, current: &DeploySnapshot) -> Vec<Event> {
    current
        .deploys
        .iter()
        .filter(|deploy| {
            previous
                .find(&deploy.key)
                .is_none_or(|p| p.status != deploy.status)
        })
        .map(deploy_event)
        .collect()
}

fn deploy_event(deploy: &Deploy) -> Event {
    let source = deploy.source.label();
    let (severity, message) = match deploy.status {
        DeployStatus::InProgress => (
            Severity::Info,
            format!("deploy of {} started ({source})", deploy.project),
        ),
        DeployStatus::Success => (
            Severity::Info,
            format!(
                "deploy of {} finished ({source}): {}",
                deploy.project, deploy.detail
            ),
        ),
        DeployStatus::Failed => {
            let stage = deploy
                .current_stage()
                .map(|s| s.name.clone())
                .unwrap_or_default();
            (
                Severity::Critical,
                format!(
                    "deploy of {} failed at stage {stage} ({source})",
                    deploy.project
                ),
            )
        }
    };
    Event::new(ID, severity, message)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn now() -> DateTime<Utc> {
        DateTime::from_timestamp(1_757_900_000, 0).expect("now")
    }

    #[test]
    fn full_fixture_merges_all_sources() {
        let raw = include_str!("../../fixtures/deploy/collect.txt");
        let state = build_state(raw, None, now());
        let sources: Vec<Source> = state.snapshot.deploys.iter().map(|d| d.source).collect();
        assert!(sources.contains(&Source::Compose));
        assert!(sources.contains(&Source::Systemd));
        assert!(sources.contains(&Source::LogFile));
        assert!(state.snapshot.runner.is_some());
        assert!(
            state
                .snapshot
                .deploys
                .windows(2)
                .all(|w| w[0].started_at >= w[1].started_at)
        );
    }

    #[test]
    fn status_change_raises_event_and_unchanged_is_silent() {
        let raw = include_str!("../../fixtures/deploy/collect.txt");
        let first = build_state(raw, None, now());
        let second = build_state(raw, Some(&first), now());
        assert!(events_between(&first.snapshot, &second.snapshot).is_empty());
        let mut changed = second.snapshot.clone();
        if let Some(deploy) = changed
            .deploys
            .iter_mut()
            .find(|d| d.source == Source::Compose)
        {
            deploy.status = DeployStatus::InProgress;
        }
        let events = events_between(&first.snapshot, &changed);
        assert_eq!(events.len(), 1);
        assert!(events[0].message.contains("started"));
    }
}
