use sib_core::ModuleError;

use super::model::{Gpu, GpuProcess, GpuSnapshot, Vendor};
use crate::common::sections::Sections;

const MIB: u64 = 1024 * 1024;

pub fn gpu_snapshot(raw: &str) -> Result<GpuSnapshot, ModuleError> {
    let sections = Sections::parse(raw);
    let mut gpus = nvidia(
        sections.get_or_empty("nvidia"),
        sections.get_or_empty("nvidia_apps"),
    );
    gpus.extend(amd(sections.get_or_empty("amd")));
    if gpus.is_empty() {
        return Err(ModuleError::Parse("no GPU in the output".to_owned()));
    }
    Ok(GpuSnapshot { gpus })
}

fn nvidia(raw: &str, apps_raw: &str) -> Vec<Gpu> {
    let mut gpus: Vec<Gpu> = raw.lines().filter_map(nvidia_line).collect();
    for line in apps_raw.lines() {
        let fields: Vec<&str> = line.split(',').map(str::trim).collect();
        if fields.len() < 4 {
            continue;
        }
        let (Ok(index), Ok(pid), Ok(memory)) = (
            fields[0].parse::<u32>(),
            fields[1].parse::<u32>(),
            fields[3].parse::<u64>(),
        ) else {
            continue;
        };
        if let Some(gpu) = gpus.iter_mut().find(|g| g.index == index) {
            gpu.processes.push(GpuProcess {
                pid,
                name: fields[2].to_owned(),
                memory_bytes: memory * MIB,
            });
        }
    }
    gpus
}

fn nvidia_line(line: &str) -> Option<Gpu> {
    let fields: Vec<&str> = line.split(',').map(str::trim).collect();
    if fields.len() < 8 {
        return None;
    }
    Some(Gpu {
        index: fields[0].parse().ok()?,
        name: fields[1].to_owned(),
        vendor: Vendor::Nvidia,
        utilization_pct: fields[2].parse().ok()?,
        memory_used: fields[3].parse::<u64>().ok()? * MIB,
        memory_total: fields[4].parse::<u64>().ok()? * MIB,
        temperature_c: fields[5].parse().ok(),
        power_w: fields[6].parse().ok(),
        power_limit_w: fields[7].parse().ok(),
        processes: Vec::new(),
    })
}

fn amd(raw: &str) -> Vec<Gpu> {
    raw.lines()
        .enumerate()
        .filter_map(|(index, line)| amd_line(index as u32, line))
        .collect()
}

fn amd_line(index: u32, line: &str) -> Option<Gpu> {
    let fields: Vec<&str> = line.split('\t').map(str::trim).collect();
    if fields.len() < 4 {
        return None;
    }
    let card = fields[0]
        .trim_end_matches("/device")
        .rsplit('/')
        .next()
        .unwrap_or("card");
    let name = fields
        .get(6)
        .filter(|n| !n.is_empty())
        .map(|n| (*n).to_owned())
        .unwrap_or_else(|| format!("AMD GPU {card}"));
    Some(Gpu {
        index,
        name,
        vendor: Vendor::Amd,
        utilization_pct: fields[1].parse().ok()?,
        memory_used: fields[2].parse().ok()?,
        memory_total: fields[3].parse().ok()?,
        temperature_c: fields
            .get(4)
            .and_then(|t| t.parse::<f64>().ok())
            .map(|t| t / 1000.0),
        power_w: fields
            .get(5)
            .and_then(|p| p.parse::<f64>().ok())
            .map(|p| p / 1_000_000.0),
        power_limit_w: None,
        processes: Vec::new(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn nvidia_fixture_gives_two_gpus_with_processes() {
        let raw = include_str!("../../fixtures/gpu/nvidia.txt");
        let snapshot = gpu_snapshot(raw).expect("snapshot");
        assert_eq!(snapshot.gpus.len(), 2);
        let first = &snapshot.gpus[0];
        assert_eq!(first.name, "NVIDIA A10");
        assert_eq!(first.utilization_pct, 37.0);
        assert_eq!(first.memory_total, 23028 * MIB);
        assert_eq!(first.temperature_c, Some(61.0));
        assert_eq!(first.power_limit_w, Some(150.0));
        assert_eq!(first.processes.len(), 2);
        assert_eq!(first.processes[0].name, "python3");
        assert!(snapshot.gpus[1].processes.is_empty());
    }

    #[test]
    fn amd_fixture_converts_sysfs_units() {
        let raw = include_str!("../../fixtures/gpu/amd.txt");
        let snapshot = gpu_snapshot(raw).expect("snapshot");
        let gpu = &snapshot.gpus[0];
        assert_eq!(gpu.vendor, Vendor::Amd);
        assert_eq!(gpu.utilization_pct, 12.0);
        assert_eq!(gpu.temperature_c, Some(48.0));
        assert_eq!(gpu.power_w, Some(31.5));
        assert!((gpu.memory_pct() - 25.0).abs() < 0.01);
        assert_eq!(gpu.name, "Radeon RX 7900");
    }

    #[test]
    fn empty_output_is_error() {
        assert!(gpu_snapshot("###nvidia\n###amd\n").is_err());
    }
}
