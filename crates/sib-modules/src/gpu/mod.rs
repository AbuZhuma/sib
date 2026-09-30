mod model;
mod parse;

use async_trait::async_trait;
use sib_core::{
    Availability, CollectContext, Event, Module, ModuleError, ModuleId, ModuleSettings, Sample,
    Schedule, Severity, Snapshot, Transport,
};

pub use model::{Gpu, GpuProcess, GpuSnapshot, Vendor};

use crate::common::{detect, sections};

pub const ID: ModuleId = ModuleId("gpu");
pub const KEY_MAX_UTIL: &str = "gpu.util_pct";

const TEMPERATURE_WARNING: f64 = 85.0;
const MEMORY_WARNING_PCT: f64 = 95.0;

const SCRIPT_PARTS: [(&str, &str); 3] = [
    (
        "nvidia",
        "nvidia-smi --query-gpu=index,name,utilization.gpu,memory.used,memory.total,temperature.gpu,power.draw,power.limit --format=csv,noheader,nounits",
    ),
    (
        "nvidia_apps",
        "map=$(nvidia-smi --query-gpu=index,pci.bus_id --format=csv,noheader,nounits); nvidia-smi --query-compute-apps=gpu_bus_id,pid,process_name,used_memory --format=csv,noheader,nounits | while IFS=, read -r bus pid name mem; do idx=$(echo \"$map\" | grep -i \"$(echo $bus | tr -d ' ')\" | cut -d, -f1); echo \"$idx,$pid,$name,$mem\"; done",
    ),
    (
        "amd",
        "for c in /sys/class/drm/card[0-9]*/device; do [ -r \"$c/gpu_busy_percent\" ] || continue; printf '%s\\t%s\\t%s\\t%s\\t%s\\t%s\\t%s\\n' \"$c\" \"$(cat $c/gpu_busy_percent)\" \"$(cat $c/mem_info_vram_used 2>/dev/null)\" \"$(cat $c/mem_info_vram_total 2>/dev/null)\" \"$(cat $c/hwmon/hwmon*/temp1_input 2>/dev/null | head -1)\" \"$(cat $c/hwmon/hwmon*/power1_average 2>/dev/null | head -1)\" \"$(cat $c/product_name 2>/dev/null)\"; done",
    ),
];

pub struct GpuModule;

#[async_trait]
impl Module for GpuModule {
    fn id(&self) -> ModuleId {
        ID
    }

    fn title(&self) -> &'static str {
        "GPU"
    }

    fn schedule(&self) -> Schedule {
        Schedule::Fast
    }

    async fn detect(
        &self,
        transport: &dyn Transport,
        _settings: &ModuleSettings,
    ) -> Result<Availability, ModuleError> {
        detect::require(
            transport,
            "command -v nvidia-smi || ls /sys/class/drm/card*/device/gpu_busy_percent",
            "нет nvidia-smi и gpu_busy_percent",
        )
        .await
    }

    async fn collect(
        &self,
        transport: &dyn Transport,
        context: &CollectContext,
    ) -> Result<Snapshot, ModuleError> {
        let output = transport.exec(&sections::script(&SCRIPT_PARTS)).await?;
        let snapshot = parse::gpu_snapshot(&output.stdout)?;
        let events = context
            .previous::<GpuSnapshot>()
            .map(|(previous, _)| events_between(previous, &snapshot))
            .unwrap_or_default();
        let samples = samples(&snapshot);
        Ok(Snapshot::new(snapshot)
            .with_samples(samples)
            .with_events(events))
    }
}

pub fn gpu_key(index: u32, metric: &str) -> String {
    format!("gpu.{index}.{metric}")
}

fn samples(snapshot: &GpuSnapshot) -> Vec<Sample> {
    let mut samples = vec![Sample::new(KEY_MAX_UTIL, snapshot.max_utilization())];
    for gpu in &snapshot.gpus {
        samples.push(Sample::new(
            gpu_key(gpu.index, "util_pct"),
            gpu.utilization_pct,
        ));
        samples.push(Sample::new(gpu_key(gpu.index, "mem_pct"), gpu.memory_pct()));
        if let Some(temperature) = gpu.temperature_c {
            samples.push(Sample::new(gpu_key(gpu.index, "temp_c"), temperature));
        }
        if let Some(power) = gpu.power_w {
            samples.push(Sample::new(gpu_key(gpu.index, "power_w"), power));
        }
    }
    samples
}

fn events_between(previous: &GpuSnapshot, current: &GpuSnapshot) -> Vec<Event> {
    let mut events = Vec::new();
    for gpu in &current.gpus {
        let before = previous.gpus.iter().find(|g| g.index == gpu.index);
        let was_hot =
            before.is_some_and(|g| g.temperature_c.is_some_and(|t| t >= TEMPERATURE_WARNING));
        if gpu.temperature_c.is_some_and(|t| t >= TEMPERATURE_WARNING) && !was_hot {
            events.push(Event::new(
                ID,
                Severity::Warning,
                format!(
                    "GPU {} перегрев: {:.0}°C",
                    gpu.name,
                    gpu.temperature_c.unwrap_or(0.0)
                ),
            ));
        }
        let was_full = before.is_some_and(|g| g.memory_pct() >= MEMORY_WARNING_PCT);
        if gpu.memory_pct() >= MEMORY_WARNING_PCT && !was_full {
            events.push(Event::new(
                ID,
                Severity::Warning,
                format!("память GPU {} занята на {:.0}%", gpu.name, gpu.memory_pct()),
            ));
        }
    }
    events
}
