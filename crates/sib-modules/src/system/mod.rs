mod model;
mod parse;

use async_trait::async_trait;
use sib_core::{
    Availability, CollectContext, Module, ModuleError, ModuleId, ModuleSettings, Schedule,
    Snapshot, Transport,
};

pub use model::{LoadAverage, SystemInfo};

use crate::common::sections;

pub const ID: ModuleId = ModuleId("system");

const SCRIPT_PARTS: [(&str, &str); 11] = [
    ("hostname", "hostname"),
    ("os", "cat /etc/os-release"),
    ("kernel", "uname -r"),
    ("arch", "uname -m"),
    ("uptime", "cat /proc/uptime"),
    ("loadavg", "cat /proc/loadavg"),
    (
        "cpuinfo",
        "grep -m1 'model name' /proc/cpuinfo; grep -c '^processor' /proc/cpuinfo",
    ),
    ("meminfo", "grep -E '^(MemTotal|SwapTotal):' /proc/meminfo"),
    ("virt", "systemd-detect-virt"),
    (
        "tz",
        "cat /etc/timezone || timedatectl show -p Timezone --value",
    ),
    ("date", "date -u +%s"),
];

pub struct SystemModule;

#[async_trait]
impl Module for SystemModule {
    fn id(&self) -> ModuleId {
        ID
    }

    fn title(&self) -> &'static str {
        "Система"
    }

    fn schedule(&self) -> Schedule {
        Schedule::Slow
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
        _context: &CollectContext,
    ) -> Result<Snapshot, ModuleError> {
        let output = transport.exec(&sections::script(&SCRIPT_PARTS)).await?;
        let info = parse::system_info(&output.stdout)?;
        Ok(Snapshot::new(info))
    }
}
