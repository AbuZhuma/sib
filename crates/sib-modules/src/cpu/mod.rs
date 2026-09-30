mod model;
mod parse;

use async_trait::async_trait;
use sib_core::{
    Availability, CollectContext, Module, ModuleError, ModuleId, ModuleSettings, Sample, Schedule,
    Snapshot, Transport,
};

pub use model::{CpuSnapshot, CpuTimes, CpuUsage};

use crate::common::sections;

pub const ID: ModuleId = ModuleId("cpu");
pub const KEY_TOTAL: &str = "cpu.total";
pub const KEY_USER: &str = "cpu.user";
pub const KEY_SYSTEM: &str = "cpu.system";
pub const KEY_IOWAIT: &str = "cpu.iowait";
pub const KEY_STEAL: &str = "cpu.steal";
pub const KEY_TEMPERATURE: &str = "cpu.temperature";

const SCRIPT_PARTS: [(&str, &str); 5] = [
    ("stat", "cat /proc/stat"),
    ("pressure", "cat /proc/pressure/cpu"),
    (
        "freq",
        "grep 'cpu MHz' /proc/cpuinfo | awk '{s+=$4; n++} END {if (n) printf \"%.0f\", s/n}'",
    ),
    (
        "thermal",
        "for t in /sys/class/thermal/thermal_zone*; do printf '%s %s\\n' \"$(cat $t/type)\" \"$(cat $t/temp)\"; done",
    ),
    (
        "hwmon",
        "for h in /sys/class/hwmon/hwmon*; do printf '%s %s\\n' \"$(cat $h/name)\" \"$(cat $h/temp1_input)\"; done",
    ),
];

pub struct CpuModule;

#[async_trait]
impl Module for CpuModule {
    fn id(&self) -> ModuleId {
        ID
    }

    fn title(&self) -> &'static str {
        "CPU"
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
        let mut snapshot = parse::cpu_snapshot(&output.stdout)?;
        if let Some((previous, _)) = context.previous::<CpuSnapshot>() {
            snapshot.usage = model::usage_between(&previous.times, &snapshot.times);
        }
        let samples = samples(&snapshot);
        Ok(Snapshot::new(snapshot).with_samples(samples))
    }
}

fn samples(snapshot: &CpuSnapshot) -> Vec<Sample> {
    let mut samples = Vec::new();
    if let Some(usage) = &snapshot.usage {
        samples.push(Sample::new(KEY_TOTAL, usage.total.busy));
        samples.push(Sample::new(KEY_USER, usage.total.user));
        samples.push(Sample::new(KEY_SYSTEM, usage.total.system));
        samples.push(Sample::new(KEY_IOWAIT, usage.total.iowait));
        samples.push(Sample::new(KEY_STEAL, usage.total.steal));
        for (index, core) in usage.cores.iter().enumerate() {
            samples.push(Sample::new(core_key(index), core.busy));
        }
    }
    if let Some(temperature) = snapshot.temperature_c {
        samples.push(Sample::new(KEY_TEMPERATURE, temperature));
    }
    samples
}

pub fn core_key(index: usize) -> String {
    format!("cpu.core.{index}")
}
