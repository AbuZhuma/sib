use sib_core::ModuleError;

use super::model::{CpuSnapshot, CpuTimes};
use crate::common::pressure;
use crate::common::sections::Sections;

const CPU_SENSOR_NAMES: [&str; 6] = [
    "coretemp",
    "k10temp",
    "cpu_thermal",
    "x86_pkg_temp",
    "cpu-thermal",
    "acpitz",
];
const MILLIDEGREES: f64 = 1000.0;

pub fn cpu_snapshot(raw: &str) -> Result<CpuSnapshot, ModuleError> {
    let sections = Sections::parse(raw);
    Ok(CpuSnapshot {
        times: times(sections.get_or_empty("stat"))?,
        usage: None,
        pressure: pressure::parse(sections.get_or_empty("pressure")),
        frequency_mhz: sections.get_or_empty("freq").trim().parse().ok(),
        temperature_c: temperature(
            sections.get_or_empty("hwmon"),
            sections.get_or_empty("thermal"),
        ),
    })
}

fn times(raw: &str) -> Result<Vec<CpuTimes>, ModuleError> {
    let parsed: Vec<CpuTimes> = raw
        .lines()
        .filter(|line| line.starts_with("cpu"))
        .filter_map(times_line)
        .collect();
    if parsed.is_empty() {
        return Err(ModuleError::Parse("нет строк cpu в /proc/stat".to_owned()));
    }
    Ok(parsed)
}

fn times_line(line: &str) -> Option<CpuTimes> {
    let mut fields = line
        .split_whitespace()
        .skip(1)
        .map(|v| v.parse::<u64>().ok());
    let mut next = || fields.next().flatten();
    Some(CpuTimes {
        user: next()?,
        nice: next()?,
        system: next()?,
        idle: next()?,
        iowait: next().unwrap_or(0),
        irq: next().unwrap_or(0),
        softirq: next().unwrap_or(0),
        steal: next().unwrap_or(0),
    })
}

fn temperature(hwmon: &str, thermal: &str) -> Option<f64> {
    let candidates = hwmon.lines().chain(thermal.lines()).filter_map(sensor_line);
    let mut fallback = None;
    for (name, value) in candidates {
        if CPU_SENSOR_NAMES.iter().any(|known| name.starts_with(known)) {
            return Some(value);
        }
        fallback.get_or_insert(value);
    }
    fallback
}

fn sensor_line(line: &str) -> Option<(&str, f64)> {
    let (name, value) = line.rsplit_once(' ')?;
    let millidegrees = value.trim().parse::<f64>().ok()?;
    Some((name.trim(), millidegrees / MILLIDEGREES))
}

#[cfg(test)]
mod tests {
    use super::*;

    const FEDORA: &str = include_str!("../../fixtures/cpu/fedora.txt");
    const VPS: &str = include_str!("../../fixtures/cpu/vps.txt");

    #[test]
    fn fedora_fixture_has_total_and_cores() {
        let snapshot = cpu_snapshot(FEDORA).expect("parse");
        assert_eq!(snapshot.core_count(), 2);
        assert_eq!(snapshot.times[0].user, 1891826);
        assert_eq!(snapshot.frequency_mhz, Some(1849.0));
        assert_eq!(snapshot.temperature_c, Some(48.0));
        assert!(snapshot.pressure.is_some());
    }

    #[test]
    fn vps_fixture_without_sensors_has_no_temperature() {
        let snapshot = cpu_snapshot(VPS).expect("parse");
        assert_eq!(snapshot.temperature_c, None);
        assert_eq!(snapshot.times[0].steal, 156);
        assert_eq!(snapshot.pressure, None);
    }

    #[test]
    fn missing_stat_is_parse_error() {
        assert!(matches!(
            cpu_snapshot("###stat\n"),
            Err(ModuleError::Parse(_))
        ));
    }
}
