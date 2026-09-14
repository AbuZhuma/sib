use asiba_core::ModuleError;

use super::model::{DeviceIo, DiskSnapshot, Filesystem, IoCounters};
use crate::common::pressure;
use crate::common::sections::Sections;

pub fn disk_snapshot(raw: &str) -> Result<DiskSnapshot, ModuleError> {
    let sections = Sections::parse(raw);
    let mut filesystems = filesystems(sections.get_or_empty("df"))?;
    apply_inodes(&mut filesystems, sections.get_or_empty("inodes"));
    Ok(DiskSnapshot {
        filesystems,
        devices: devices(sections.get_or_empty("diskstats")),
        pressure: pressure::parse(sections.get_or_empty("pressure")),
    })
}

fn filesystems(raw: &str) -> Result<Vec<Filesystem>, ModuleError> {
    let parsed: Vec<Filesystem> = raw.lines().skip(1).filter_map(filesystem_line).collect();
    if parsed.is_empty() {
        return Err(ModuleError::Parse("пустой вывод df".to_owned()));
    }
    Ok(parsed)
}

fn filesystem_line(line: &str) -> Option<Filesystem> {
    let fields: Vec<&str> = line.split_whitespace().collect();
    if fields.len() < 6 {
        return None;
    }
    Some(Filesystem {
        device: fields[0].to_owned(),
        mount: fields[5..].join(" "),
        total_bytes: fields[1].parse().ok()?,
        used_bytes: fields[2].parse().ok()?,
        available_bytes: fields[3].parse().ok()?,
        inodes_total: 0,
        inodes_used: 0,
    })
}

fn apply_inodes(filesystems: &mut [Filesystem], raw: &str) {
    for line in raw.lines().skip(1) {
        let fields: Vec<&str> = line.split_whitespace().collect();
        if fields.len() < 6 {
            continue;
        }
        let mount = fields[5..].join(" ");
        let Some(fs) = filesystems.iter_mut().find(|fs| fs.mount == mount) else {
            continue;
        };
        fs.inodes_total = fields[1].parse().unwrap_or(0);
        fs.inodes_used = fields[2].parse().unwrap_or(0);
    }
}

fn devices(raw: &str) -> Vec<DeviceIo> {
    raw.lines()
        .filter_map(device_line)
        .filter(|d| is_whole_device(&d.name))
        .collect()
}

fn device_line(line: &str) -> Option<DeviceIo> {
    let fields: Vec<&str> = line.split_whitespace().collect();
    if fields.len() < 13 {
        return None;
    }
    let number = |index: usize| fields[index].parse::<u64>().ok();
    Some(DeviceIo {
        name: fields[2].to_owned(),
        counters: IoCounters {
            reads: number(3)?,
            read_sectors: number(5)?,
            writes: number(7)?,
            write_sectors: number(9)?,
            io_ticks_ms: number(12)?,
        },
        rates: None,
    })
}

fn is_whole_device(name: &str) -> bool {
    let is_partition = |prefix: &str| {
        name.strip_prefix(prefix)
            .is_some_and(|rest| rest.chars().any(|c| c.is_ascii_digit()))
    };
    let is_nvme_partition = name.starts_with("nvme") && name.contains('p');
    let is_mmc_partition = name.starts_with("mmcblk") && name.contains('p');
    let ignored = name.starts_with("loop") || name.starts_with("ram") || name.starts_with("zram");
    !ignored
        && !is_nvme_partition
        && !is_mmc_partition
        && !is_partition("sd")
        && !is_partition("vd")
        && !is_partition("xvd")
        && !is_partition("hd")
}

#[cfg(test)]
mod tests {
    use super::*;

    const FEDORA: &str = include_str!("../../fixtures/disk/fedora.txt");
    const VPS: &str = include_str!("../../fixtures/disk/vps.txt");

    #[test]
    fn fedora_fixture_parses_filesystems_and_whole_devices() {
        let snapshot = disk_snapshot(FEDORA).expect("parse");
        let mounts: Vec<&str> = snapshot
            .filesystems
            .iter()
            .map(|f| f.mount.as_str())
            .collect();
        assert_eq!(mounts, vec!["/", "/home", "/boot"]);
        let devices: Vec<&str> = snapshot.devices.iter().map(|d| d.name.as_str()).collect();
        assert_eq!(devices, vec!["nvme0n1"]);
        assert_eq!(snapshot.filesystems[2].inodes_total, 131072);
    }

    #[test]
    fn vps_fixture_keeps_vda_and_drops_partitions() {
        let snapshot = disk_snapshot(VPS).expect("parse");
        let devices: Vec<&str> = snapshot.devices.iter().map(|d| d.name.as_str()).collect();
        assert_eq!(devices, vec!["vda"]);
        assert_eq!(snapshot.pressure, None);
    }

    #[test]
    fn whole_device_detection_handles_common_names() {
        assert!(is_whole_device("sda"));
        assert!(!is_whole_device("sda1"));
        assert!(is_whole_device("nvme0n1"));
        assert!(!is_whole_device("nvme0n1p3"));
        assert!(is_whole_device("md0"));
        assert!(is_whole_device("dm-0"));
        assert!(!is_whole_device("loop0"));
    }
}
