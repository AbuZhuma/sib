mod actions;
mod model;
mod parse;

use asiba_core::{
    ActionOutcome, ActionRequest, ActionSpec, Availability, CollectContext, Module, ModuleError,
    ModuleId, Sample, Schedule, Snapshot, Transport,
};
use async_trait::async_trait;

pub use actions::{ACTION_KILL, ACTION_TERMINATE, SPEC_KILL, SPEC_TERMINATE};
pub use model::{Process, ProcessSnapshot};

use crate::common::sections;

pub const ID: ModuleId = ModuleId("processes");
pub const KEY_COUNT: &str = "processes.count";
pub const KEY_RUNNING: &str = "processes.running";
pub const KEY_ZOMBIES: &str = "processes.zombies";

const SCRIPT_PARTS: [(&str, &str); 8] = [
    ("stat", "cat /proc/[0-9]*/stat"),
    (
        "cmd",
        "for p in /proc/[0-9]*; do printf '%s\\t' \"${p#/proc/}\"; tr '\\0' ' ' < \"$p/cmdline\"; echo; done",
    ),
    (
        "io",
        "for p in /proc/[0-9]*; do printf '%s\\t' \"${p#/proc/}\"; tr '\\n' ' ' < \"$p/io\"; echo; done",
    ),
    ("users", "ps -eo pid=,user="),
    ("hz", "getconf CLK_TCK"),
    ("pagesize", "getconf PAGESIZE"),
    ("uptime", "cat /proc/uptime"),
    ("memtotal", "grep MemTotal /proc/meminfo"),
];

pub struct ProcessesModule;

#[async_trait]
impl Module for ProcessesModule {
    fn id(&self) -> ModuleId {
        ID
    }

    fn title(&self) -> &'static str {
        "Процессы"
    }

    fn schedule(&self) -> Schedule {
        Schedule::Fast
    }

    async fn detect(&self, _transport: &dyn Transport) -> Result<Availability, ModuleError> {
        Ok(Availability::Available)
    }

    async fn collect(
        &self,
        transport: &dyn Transport,
        context: &CollectContext,
    ) -> Result<Snapshot, ModuleError> {
        let output = transport.exec(&sections::script(&SCRIPT_PARTS)).await?;
        let mut snapshot = parse::process_snapshot(&output.stdout)?;
        if let Some((previous, elapsed)) = context.previous::<ProcessSnapshot>() {
            model::fill_rates(&mut snapshot, previous, elapsed);
        }
        let samples = vec![
            Sample::new(KEY_COUNT, snapshot.processes.len() as f64),
            Sample::new(KEY_RUNNING, snapshot.running_count() as f64),
            Sample::new(KEY_ZOMBIES, snapshot.zombie_count() as f64),
        ];
        Ok(Snapshot::new(snapshot).with_samples(samples))
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
