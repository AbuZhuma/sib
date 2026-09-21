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
pub use model::{ServicesSnapshot, Timer, Unit, UnitOrigin};

use crate::common::{detect, sections};

pub const ID: ModuleId = ModuleId("services");
pub const KEY_FAILED: &str = "services.failed";
pub const KEY_ACTIVE: &str = "services.active";
pub const QUERY_JOURNAL: &str = "journal";

const JOURNAL_LINES: u32 = 200;
const SCRIPT_PARTS: [(&str, &str); 3] = [
    (
        "units",
        "systemctl list-units --type=service --all --plain --no-legend --no-pager",
    ),
    (
        "show",
        "TZ=UTC systemctl show '*.service' -p Id -p NRestarts -p MainPID -p ActiveEnterTimestamp -p FragmentPath -p WorkingDirectory -p Result --no-pager",
    ),
    (
        "timers",
        "TZ=UTC systemctl list-timers --all --no-legend --no-pager -o json",
    ),
];

pub struct ServicesModule;

#[async_trait]
impl Module for ServicesModule {
    fn id(&self) -> ModuleId {
        ID
    }

    fn title(&self) -> &'static str {
        "Сервисы"
    }

    fn schedule(&self) -> Schedule {
        Schedule::Normal
    }

    async fn detect(
        &self,
        transport: &dyn Transport,
        _settings: &ModuleSettings,
    ) -> Result<Availability, ModuleError> {
        detect::require(transport, "command -v systemctl", "нет systemd").await
    }

    async fn collect(
        &self,
        transport: &dyn Transport,
        context: &CollectContext,
    ) -> Result<Snapshot, ModuleError> {
        let output = transport.exec(&sections::script(&SCRIPT_PARTS)).await?;
        let snapshot = parse::services_snapshot(&output.stdout)?;
        let events = context
            .previous::<ServicesSnapshot>()
            .map(|(previous, _)| events_between(previous, &snapshot))
            .unwrap_or_default();
        let samples = vec![
            Sample::new(KEY_FAILED, snapshot.failed().count() as f64),
            Sample::new(KEY_ACTIVE, snapshot.active_count() as f64),
        ];
        Ok(Snapshot::new(snapshot)
            .with_samples(samples)
            .with_events(events))
    }

    async fn query(
        &self,
        transport: &dyn Transport,
        request: &QueryRequest,
    ) -> Result<QueryResponse, ModuleError> {
        if request.kind != QUERY_JOURNAL {
            return Err(ModuleError::UnsupportedQuery(request.kind.clone()));
        }
        let command = format!(
            "journalctl -u {} -n {JOURNAL_LINES} --no-pager -o short-iso",
            shell_quote(&request.target)
        );
        let output = transport.exec(&command).await?;
        let text = if output.stdout.trim().is_empty() {
            output.stderr
        } else {
            output.stdout
        };
        Ok(QueryResponse {
            title: format!("journal {}", request.target),
            text,
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

fn events_between(previous: &ServicesSnapshot, current: &ServicesSnapshot) -> Vec<Event> {
    let mut events = Vec::new();
    for unit in &current.units {
        let Some(before) = previous.units.iter().find(|u| u.name == unit.name) else {
            continue;
        };
        if unit.is_failed() && !before.is_failed() {
            let message = format!("{} перешёл в failed ({})", unit.name, unit.result);
            events.push(Event::new(ID, Severity::Critical, message));
        }
        if unit.restarts > before.restarts {
            let message = format!("{} перезапущен ({} раз)", unit.name, unit.restarts);
            events.push(Event::new(ID, Severity::Warning, message));
        }
    }
    events
}
