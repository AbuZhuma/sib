mod actions;
mod model;
mod parse;

use asiba_core::transport::shell_quote;
use asiba_core::{
    ActionOutcome, ActionRequest, ActionSpec, Availability, CollectContext, Event, Module,
    ModuleError, ModuleId, ModuleSettings, QueryRequest, QueryResponse, Sample, Schedule, Severity,
    Snapshot, Transport,
};
use async_trait::async_trait;

pub use actions::{ACTION_RESTART, ACTION_START, ACTION_STOP, SPEC_RESTART, SPEC_START, SPEC_STOP};
pub use model::{Container, ContainerStats, DockerSnapshot, Image};

use crate::common::sections;

pub const ID: ModuleId = ModuleId("docker");
pub const KEY_RUNNING: &str = "docker.running";
pub const KEY_STOPPED: &str = "docker.stopped";
pub const QUERY_LOGS: &str = "logs";

const LOG_LINES: u32 = 300;
const BINARY: &str = "D=$(command -v docker || command -v podman)";
const PS_FORMAT: &str = "{{.ID}}\\t{{.Names}}\\t{{.Image}}\\t{{.State}}\\t{{.Status}}\\t{{.Ports}}";
const STATS_FORMAT: &str =
    "{{.ID}}\\t{{.Name}}\\t{{.CPUPerc}}\\t{{.MemUsage}}\\t{{.NetIO}}\\t{{.BlockIO}}\\t{{.PIDs}}";
const IMAGES_FORMAT: &str = "{{.ID}}\\t{{.Repository}}\\t{{.Tag}}\\t{{.Size}}\\t{{.CreatedSince}}";
const INSPECT_FORMAT: &str = "{{.Id}}\\t{{.Name}}\\t{{.RestartCount}}\\t{{if .State.Health}}{{.State.Health.Status}}{{end}}\\t{{.HostConfig.RestartPolicy.Name}}\\t{{.State.ExitCode}}\\t{{.State.StartedAt}}\\t{{index .Config.Labels \"com.docker.compose.project\"}}\\t{{index .Config.Labels \"com.docker.compose.service\"}}\\t{{index .Config.Labels \"com.docker.compose.project.working_dir\"}}";

pub struct DockerModule;

fn script() -> String {
    let parts = [
        (
            "version",
            "$D version --format '{{.Server.Version}}'".to_owned(),
        ),
        ("ps", format!("$D ps -a --format '{PS_FORMAT}'")),
        (
            "stats",
            format!("$D stats --no-stream --format '{STATS_FORMAT}'"),
        ),
        ("images", format!("$D images --format '{IMAGES_FORMAT}'")),
        (
            "inspect",
            format!(
                "ids=$($D ps -aq); [ -n \"$ids\" ] && $D inspect --format '{INSPECT_FORMAT}' $ids"
            ),
        ),
        ("volumes", "$D volume ls -q | wc -l".to_owned()),
        ("networks", "$D network ls --format '{{.Name}}'".to_owned()),
    ];
    let borrowed: Vec<(&str, &str)> = parts.iter().map(|(n, c)| (*n, c.as_str())).collect();
    format!("{BINARY}; {}", sections::script(&borrowed))
}

#[async_trait]
impl Module for DockerModule {
    fn id(&self) -> ModuleId {
        ID
    }

    fn title(&self) -> &'static str {
        "Docker"
    }

    fn schedule(&self) -> Schedule {
        Schedule::Normal
    }

    async fn detect(
        &self,
        transport: &dyn Transport,
        _settings: &ModuleSettings,
    ) -> Result<Availability, ModuleError> {
        let probe = format!("{BINARY}; [ -n \"$D\" ] || exit 3; $D ps -q >/dev/null");
        let output = transport.exec(&probe).await?;
        match output.exit_code {
            0 => Ok(Availability::Available),
            3 => Ok(Availability::unavailable("нет docker/podman")),
            _ => Ok(Availability::unavailable(
                "нет доступа к docker: добавьте пользователя в группу docker",
            )),
        }
    }

    async fn collect(
        &self,
        transport: &dyn Transport,
        context: &CollectContext,
    ) -> Result<Snapshot, ModuleError> {
        let output = transport.exec(&script()).await?;
        let snapshot = parse::docker_snapshot(&output.stdout)?;
        let events = context
            .previous::<DockerSnapshot>()
            .map(|(previous, _)| events_between(previous, &snapshot))
            .unwrap_or_default();
        let mut samples = vec![
            Sample::new(KEY_RUNNING, snapshot.running_count() as f64),
            Sample::new(KEY_STOPPED, snapshot.stopped_count() as f64),
        ];
        for container in snapshot.containers.iter().filter(|c| c.stats.is_some()) {
            if let Some(stats) = &container.stats {
                samples.push(Sample::new(
                    container_key(&container.name, "cpu_pct"),
                    stats.cpu_pct,
                ));
                samples.push(Sample::new(
                    container_key(&container.name, "mem_bytes"),
                    stats.mem_usage as f64,
                ));
            }
        }
        Ok(Snapshot::new(snapshot)
            .with_samples(samples)
            .with_events(events))
    }

    async fn query(
        &self,
        transport: &dyn Transport,
        request: &QueryRequest,
    ) -> Result<QueryResponse, ModuleError> {
        if request.kind != QUERY_LOGS {
            return Err(ModuleError::UnsupportedQuery(request.kind.clone()));
        }
        let command = format!(
            "{BINARY}; $D logs --tail {LOG_LINES} -t {} 2>&1",
            shell_quote(&request.target)
        );
        let output = transport.exec(&command).await?;
        Ok(QueryResponse {
            title: format!("logs {}", request.target),
            text: output.stdout,
        })
    }

    fn actions(&self) -> &'static [ActionSpec] {
        &actions::SPECS
    }

    async fn perform(
        &self,
        transport: &dyn Transport,
        request: &ActionRequest,
    ) -> Result<ActionOutcome, ModuleError> {
        actions::perform(transport, request).await
    }
}

pub fn container_key(name: &str, metric: &str) -> String {
    format!("docker.{name}.{metric}")
}

fn events_between(previous: &DockerSnapshot, current: &DockerSnapshot) -> Vec<Event> {
    let mut events = Vec::new();
    for container in &current.containers {
        let before = previous.containers.iter().find(|c| c.id == container.id);
        let was_running = before.is_some_and(|c| c.is_running());
        if was_running && !container.is_running() {
            let message = format!(
                "{} остановлен (exit {})",
                container.name, container.exit_code
            );
            let severity = if container.exit_code == 0 {
                Severity::Info
            } else {
                Severity::Critical
            };
            events.push(Event::new(ID, severity, message));
        }
        if before.is_some_and(|c| c.restart_count < container.restart_count) {
            let message = format!(
                "{} перезапущен ({} раз)",
                container.name, container.restart_count
            );
            events.push(Event::new(ID, Severity::Warning, message));
        }
        let became_unhealthy = container.health.as_deref() == Some("unhealthy")
            && before.and_then(|c| c.health.as_deref()) != Some("unhealthy");
        if became_unhealthy {
            events.push(Event::new(
                ID,
                Severity::Critical,
                format!("{} unhealthy", container.name),
            ));
        }
    }
    events
}
