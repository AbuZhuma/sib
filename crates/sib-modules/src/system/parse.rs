use std::time::Duration;

use sib_core::ModuleError;

use super::model::{LoadAverage, SystemInfo};
use crate::common::sections::Sections;

const KIB: u64 = 1024;

pub fn system_info(raw: &str) -> Result<SystemInfo, ModuleError> {
    let sections = Sections::parse(raw);
    let (cpu_model, cpu_cores) = cpu(sections.get_or_empty("cpuinfo"));
    let (mem_total_bytes, swap_total_bytes) = memory(sections.get_or_empty("meminfo"));
    Ok(SystemInfo {
        hostname: sections.get_or_empty("hostname").to_owned(),
        os_name: os_release_field(sections.get_or_empty("os"), "PRETTY_NAME")
            .unwrap_or_else(|| "Linux".to_owned()),
        os_id: os_release_field(sections.get_or_empty("os"), "ID").unwrap_or_default(),
        kernel: sections.get_or_empty("kernel").to_owned(),
        arch: sections.get_or_empty("arch").to_owned(),
        uptime: uptime(sections.get_or_empty("uptime"))?,
        load: load_average(sections.get_or_empty("loadavg"))?,
        cpu_model,
        cpu_cores,
        mem_total_bytes,
        swap_total_bytes,
        virtualization: optional(sections.get_or_empty("virt")).filter(|v| v != "none"),
        timezone: optional(sections.get_or_empty("tz")),
        clock_offset_secs: clock_offset(sections.get_or_empty("date")),
    })
}

fn optional(value: &str) -> Option<String> {
    let trimmed = value.trim();
    (!trimmed.is_empty()).then(|| trimmed.to_owned())
}

fn os_release_field(raw: &str, key: &str) -> Option<String> {
    raw.lines()
        .find_map(|line| line.strip_prefix(key)?.strip_prefix('='))
        .map(|value| value.trim().trim_matches('"').to_owned())
}

fn uptime(raw: &str) -> Result<Duration, ModuleError> {
    let seconds = raw
        .split_whitespace()
        .next()
        .and_then(|v| v.parse::<f64>().ok())
        .ok_or_else(|| ModuleError::Parse(format!("uptime: {raw:?}")))?;
    Ok(Duration::from_secs_f64(seconds))
}

fn load_average(raw: &str) -> Result<LoadAverage, ModuleError> {
    let mut parts = raw.split_whitespace().map(|v| v.parse::<f64>().ok());
    let mut next = || parts.next().flatten();
    match (next(), next(), next()) {
        (Some(one), Some(five), Some(fifteen)) => Ok(LoadAverage { one, five, fifteen }),
        _ => Err(ModuleError::Parse(format!("loadavg: {raw:?}"))),
    }
}

fn cpu(raw: &str) -> (String, u32) {
    let model = raw
        .lines()
        .find_map(|line| line.split_once(':'))
        .map(|(_, value)| value.trim().to_owned())
        .unwrap_or_default();
    let cores = raw
        .lines()
        .last()
        .and_then(|line| line.trim().parse().ok())
        .unwrap_or(0);
    (model, cores)
}

fn memory(raw: &str) -> (u64, u64) {
    let field = |key: &str| {
        raw.lines()
            .find_map(|line| line.strip_prefix(key))
            .and_then(|rest| rest.trim_start_matches(':').split_whitespace().next())
            .and_then(|v| v.parse::<u64>().ok())
            .map(|kib| kib * KIB)
            .unwrap_or(0)
    };
    (field("MemTotal"), field("SwapTotal"))
}

fn clock_offset(raw: &str) -> i64 {
    let Some(remote) = raw.trim().parse::<i64>().ok() else {
        return 0;
    };
    let local = chrono::Utc::now().timestamp();
    remote - local
}

#[cfg(test)]
mod tests {
    use super::*;

    const FEDORA: &str = include_str!("../../fixtures/system/fedora.txt");
    const DEBIAN: &str = include_str!("../../fixtures/system/debian.txt");

    #[test]
    fn fedora_fixture_parses_all_fields() {
        let info = system_info(FEDORA).expect("parse");
        assert_eq!(info.hostname, "neo");
        assert_eq!(info.os_name, "Fedora Linux 42 (Server Edition)");
        assert_eq!(info.os_id, "fedora");
        assert_eq!(info.kernel, "6.14.0-63.fc42.x86_64");
        assert_eq!(info.cpu_cores, 4);
        assert_eq!(info.cpu_model, "AMD EPYC 7B13");
        assert_eq!(info.mem_total_bytes, 8_143_268 * KIB);
        assert_eq!(info.virtualization, Some("kvm".to_owned()));
        assert_eq!(info.load.one, 0.42);
        assert_eq!(info.uptime.as_secs(), 1_234_567);
    }

    #[test]
    fn debian_fixture_without_virt_has_none() {
        let info = system_info(DEBIAN).expect("parse");
        assert_eq!(info.virtualization, None);
        assert_eq!(info.timezone, Some("Europe/Berlin".to_owned()));
        assert_eq!(info.swap_total_bytes, 0);
    }

    #[test]
    fn missing_uptime_is_parse_error() {
        assert!(matches!(
            system_info("###hostname\nx\n"),
            Err(ModuleError::Parse(_))
        ));
    }

    #[test]
    fn uptime_human_formats_days() {
        let mut info = system_info(FEDORA).expect("parse");
        info.uptime = Duration::from_secs(90_000);
        assert_eq!(info.uptime_human(), "1d 1h 0m");
    }
}
