mod model;
mod parse;

use async_trait::async_trait;
use chrono::Utc;
use sib_core::{
    Availability, CollectContext, Event, Module, ModuleError, ModuleId, ModuleSettings, Sample,
    Schedule, Severity, Snapshot, Transport,
};

pub use model::{MemorySnapshot, OOM_RECENT_WINDOW};

use crate::common::rate;
use crate::common::sections;

pub const ID: ModuleId = ModuleId("memory");
pub const KEY_USED_PCT: &str = "memory.used_pct";
pub const KEY_USED_BYTES: &str = "memory.used_bytes";
pub const KEY_AVAILABLE_BYTES: &str = "memory.available_bytes";
pub const KEY_CACHED_BYTES: &str = "memory.cached_bytes";
pub const KEY_SWAP_USED_PCT: &str = "memory.swap_used_pct";
pub const KEY_SWAP_IN_PS: &str = "memory.swap_in_ps";
pub const KEY_SWAP_OUT_PS: &str = "memory.swap_out_ps";
pub const KEY_PRESSURE_SOME10: &str = "memory.pressure_some10";

const SCRIPT_PARTS: [(&str, &str); 3] = [
    ("meminfo", "cat /proc/meminfo"),
    ("pressure", "cat /proc/pressure/memory"),
    (
        "vmstat",
        "grep -E '^(oom_kill|pswpin|pswpout) ' /proc/vmstat",
    ),
];

pub struct MemoryModule;

#[async_trait]
impl Module for MemoryModule {
    fn id(&self) -> ModuleId {
        ID
    }

    fn title(&self) -> &'static str {
        "Память"
    }

    fn schedule(&self) -> Schedule {
        Schedule::Fast
    }

    async fn detect(
        &self,
        _transport: &dyn Transport,
        _settings: &ModuleSettings,
    ) -> Result<Availability, ModuleError> {
        Ok(Availability::Available)
    }

    async fn collect(
        &self,
        transport: &dyn Transport,
        context: &CollectContext,
    ) -> Result<Snapshot, ModuleError> {
        let output = transport.exec(&sections::script(&SCRIPT_PARTS)).await?;
        let mut snapshot = parse::memory_snapshot(&output.stdout)?;
        let mut events = Vec::new();
        if let Some((previous, elapsed)) = context.previous::<MemorySnapshot>() {
            snapshot.swap_in_ps = Some(rate::per_second(
                snapshot.swap_in_pages,
                previous.swap_in_pages,
                elapsed,
            ));
            snapshot.swap_out_ps = Some(rate::per_second(
                snapshot.swap_out_pages,
                previous.swap_out_pages,
                elapsed,
            ));
            let new_kills = snapshot.oom_kills.saturating_sub(previous.oom_kills);
            snapshot.oom_kills_observed = previous.oom_kills_observed + new_kills;
            snapshot.last_oom_at = previous.last_oom_at;
            if new_kills > 0 {
                snapshot.last_oom_at = Some(Utc::now());
                events.push(oom_event(new_kills));
            }
        }
        let samples = samples(&snapshot);
        Ok(Snapshot::new(snapshot)
            .with_samples(samples)
            .with_events(events))
    }
}

fn oom_event(count: u64) -> Event {
    Event::new(
        ID,
        Severity::Critical,
        format!("OOM killer завершил процессов: {count}"),
    )
}

fn samples(snapshot: &MemorySnapshot) -> Vec<Sample> {
    let mut samples = vec![
        Sample::new(KEY_USED_PCT, snapshot.used_pct()),
        Sample::new(KEY_USED_BYTES, snapshot.used_bytes() as f64),
        Sample::new(KEY_AVAILABLE_BYTES, snapshot.available_bytes as f64),
        Sample::new(KEY_CACHED_BYTES, snapshot.cached_bytes as f64),
    ];
    if snapshot.swap_total_bytes > 0 {
        samples.push(Sample::new(KEY_SWAP_USED_PCT, snapshot.swap_used_pct()));
    }
    if let (Some(swap_in), Some(swap_out)) = (snapshot.swap_in_ps, snapshot.swap_out_ps) {
        samples.push(Sample::new(KEY_SWAP_IN_PS, swap_in));
        samples.push(Sample::new(KEY_SWAP_OUT_PS, swap_out));
    }
    if let Some(pressure) = snapshot.pressure {
        samples.push(Sample::new(KEY_PRESSURE_SOME10, pressure.some.avg10));
    }
    samples
}
