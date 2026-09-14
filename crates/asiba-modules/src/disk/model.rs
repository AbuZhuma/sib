use crate::common::pressure::Pressure;
use crate::common::rate;

const SECTOR_BYTES: u64 = 512;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Filesystem {
    pub device: String,
    pub mount: String,
    pub total_bytes: u64,
    pub used_bytes: u64,
    pub available_bytes: u64,
    pub inodes_total: u64,
    pub inodes_used: u64,
}

impl Filesystem {
    pub fn used_pct(&self) -> f64 {
        rate::percent(self.used_bytes, self.used_bytes + self.available_bytes)
    }

    pub fn inodes_used_pct(&self) -> f64 {
        rate::percent(self.inodes_used, self.inodes_total)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct IoCounters {
    pub reads: u64,
    pub read_sectors: u64,
    pub writes: u64,
    pub write_sectors: u64,
    pub io_ticks_ms: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct IoRates {
    pub read_bps: f64,
    pub write_bps: f64,
    pub read_iops: f64,
    pub write_iops: f64,
    pub util_pct: f64,
}

#[derive(Debug, Clone, PartialEq)]
pub struct DeviceIo {
    pub name: String,
    pub counters: IoCounters,
    pub rates: Option<IoRates>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct DiskSnapshot {
    pub filesystems: Vec<Filesystem>,
    pub devices: Vec<DeviceIo>,
    pub pressure: Option<Pressure>,
}

pub fn fill_rates(current: &mut DiskSnapshot, previous: &DiskSnapshot, elapsed: f64) {
    for device in &mut current.devices {
        let Some(before) = previous.devices.iter().find(|d| d.name == device.name) else {
            continue;
        };
        device.rates = Some(rates_between(&before.counters, &device.counters, elapsed));
    }
}

fn rates_between(previous: &IoCounters, current: &IoCounters, elapsed: f64) -> IoRates {
    let per_second = |now: u64, before: u64| rate::per_second(now, before, elapsed);
    let busy_ms = per_second(current.io_ticks_ms, previous.io_ticks_ms);
    IoRates {
        read_bps: per_second(current.read_sectors, previous.read_sectors) * SECTOR_BYTES as f64,
        write_bps: per_second(current.write_sectors, previous.write_sectors) * SECTOR_BYTES as f64,
        read_iops: per_second(current.reads, previous.reads),
        write_iops: per_second(current.writes, previous.writes),
        util_pct: (busy_ms / 10.0).clamp(0.0, 100.0),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rates_between_converts_sectors_to_bytes() {
        let previous = IoCounters::default();
        let current = IoCounters {
            read_sectors: 200,
            io_ticks_ms: 500,
            ..Default::default()
        };
        let rates = rates_between(&previous, &current, 2.0);
        assert_eq!(rates.read_bps, 100.0 * 512.0);
        assert_eq!(rates.util_pct, 25.0);
    }

    #[test]
    fn used_pct_ignores_reserved_blocks() {
        let fs = Filesystem {
            device: "/dev/sda1".into(),
            mount: "/".into(),
            total_bytes: 100,
            used_bytes: 30,
            available_bytes: 60,
            inodes_total: 0,
            inodes_used: 0,
        };
        assert!((fs.used_pct() - 33.333).abs() < 0.01);
    }
}
