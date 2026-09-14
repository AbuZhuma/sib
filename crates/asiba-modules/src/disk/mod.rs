mod model;
mod parse;

use asiba_core::{
    Availability, CollectContext, Module, ModuleError, ModuleId, Sample, Schedule, Snapshot,
    Transport,
};
use async_trait::async_trait;

pub use model::{DeviceIo, DiskSnapshot, Filesystem, IoRates};

use crate::common::sections;

pub const ID: ModuleId = ModuleId("disk");
pub const KEY_ROOT_USED_PCT: &str = "disk.root_used_pct";
pub const KEY_READ_BPS: &str = "disk.read_bps";
pub const KEY_WRITE_BPS: &str = "disk.write_bps";
pub const KEY_PRESSURE_SOME10: &str = "disk.pressure_some10";

const DF_FILTER: &str = "-x tmpfs -x devtmpfs -x squashfs -x overlay -x efivarfs -x fuse.portal";

pub struct DiskModule;

fn script() -> String {
    let df = format!("df -P -B1 {DF_FILTER}");
    let df_inodes = format!("df -P -i {DF_FILTER}");
    sections::script(&[
        ("df", &df),
        ("inodes", &df_inodes),
        ("diskstats", "cat /proc/diskstats"),
        ("pressure", "cat /proc/pressure/io"),
    ])
}

#[async_trait]
impl Module for DiskModule {
    fn id(&self) -> ModuleId {
        ID
    }

    fn title(&self) -> &'static str {
        "Диски"
    }

    fn schedule(&self) -> Schedule {
        Schedule::Fast
    }

    async fn detect(&self, transport: &dyn Transport) -> Result<Availability, ModuleError> {
        let output = transport.exec("command -v df").await?;
        if output.is_success() {
            return Ok(Availability::Available);
        }
        Ok(Availability::Unavailable {
            reason: "нет df".to_owned(),
        })
    }

    async fn collect(
        &self,
        transport: &dyn Transport,
        context: &CollectContext,
    ) -> Result<Snapshot, ModuleError> {
        let output = transport.exec(&script()).await?;
        let mut snapshot = parse::disk_snapshot(&output.stdout)?;
        if let Some((previous, elapsed)) = context.previous::<DiskSnapshot>() {
            model::fill_rates(&mut snapshot, previous, elapsed);
        }
        let samples = samples(&snapshot);
        Ok(Snapshot::new(snapshot).with_samples(samples))
    }
}

fn samples(snapshot: &DiskSnapshot) -> Vec<Sample> {
    let mut samples = Vec::new();
    for filesystem in &snapshot.filesystems {
        samples.push(Sample::new(
            mount_key(&filesystem.mount),
            filesystem.used_pct(),
        ));
        if filesystem.mount == "/" {
            samples.push(Sample::new(KEY_ROOT_USED_PCT, filesystem.used_pct()));
        }
    }
    let (mut read_bps, mut write_bps) = (0.0, 0.0);
    for device in &snapshot.devices {
        let Some(rates) = &device.rates else {
            continue;
        };
        read_bps += rates.read_bps;
        write_bps += rates.write_bps;
        samples.push(Sample::new(
            device_key(&device.name, "read_bps"),
            rates.read_bps,
        ));
        samples.push(Sample::new(
            device_key(&device.name, "write_bps"),
            rates.write_bps,
        ));
        samples.push(Sample::new(
            device_key(&device.name, "util_pct"),
            rates.util_pct,
        ));
    }
    if snapshot.devices.iter().any(|d| d.rates.is_some()) {
        samples.push(Sample::new(KEY_READ_BPS, read_bps));
        samples.push(Sample::new(KEY_WRITE_BPS, write_bps));
    }
    if let Some(pressure) = snapshot.pressure {
        samples.push(Sample::new(KEY_PRESSURE_SOME10, pressure.some.avg10));
    }
    samples
}

pub fn mount_key(mount: &str) -> String {
    format!("disk.fs.{mount}.used_pct")
}

pub fn device_key(device: &str, metric: &str) -> String {
    format!("disk.io.{device}.{metric}")
}
