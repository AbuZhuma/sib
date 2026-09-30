use sib_core::ModuleError;

use super::model::MemorySnapshot;
use crate::common::pressure;
use crate::common::sections::Sections;

const KIB: u64 = 1024;

pub fn memory_snapshot(raw: &str) -> Result<MemorySnapshot, ModuleError> {
    let sections = Sections::parse(raw);
    let meminfo = sections.get_or_empty("meminfo");
    let vmstat = sections.get_or_empty("vmstat");
    let total_bytes = meminfo_field(meminfo, "MemTotal")
        .ok_or_else(|| ModuleError::Parse("no MemTotal in /proc/meminfo".to_owned()))?;
    Ok(MemorySnapshot {
        total_bytes,
        free_bytes: meminfo_field(meminfo, "MemFree").unwrap_or(0),
        available_bytes: meminfo_field(meminfo, "MemAvailable").unwrap_or(0),
        buffers_bytes: meminfo_field(meminfo, "Buffers").unwrap_or(0),
        cached_bytes: meminfo_field(meminfo, "Cached").unwrap_or(0),
        shmem_bytes: meminfo_field(meminfo, "Shmem").unwrap_or(0),
        reclaimable_bytes: meminfo_field(meminfo, "SReclaimable").unwrap_or(0),
        dirty_bytes: meminfo_field(meminfo, "Dirty").unwrap_or(0),
        swap_total_bytes: meminfo_field(meminfo, "SwapTotal").unwrap_or(0),
        swap_free_bytes: meminfo_field(meminfo, "SwapFree").unwrap_or(0),
        pressure: pressure::parse(sections.get_or_empty("pressure")),
        oom_kills: vmstat_field(vmstat, "oom_kill"),
        oom_kills_observed: 0,
        last_oom_at: None,
        swap_in_pages: vmstat_field(vmstat, "pswpin"),
        swap_out_pages: vmstat_field(vmstat, "pswpout"),
        swap_in_ps: None,
        swap_out_ps: None,
    })
}

fn meminfo_field(raw: &str, key: &str) -> Option<u64> {
    raw.lines()
        .find_map(|line| line.strip_prefix(key)?.strip_prefix(':'))
        .and_then(|rest| rest.split_whitespace().next())
        .and_then(|value| value.parse::<u64>().ok())
        .map(|kib| kib * KIB)
}

fn vmstat_field(raw: &str, key: &str) -> u64 {
    raw.lines()
        .find_map(|line| line.strip_prefix(key)?.strip_prefix(' '))
        .and_then(|value| value.trim().parse().ok())
        .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;

    const FEDORA: &str = include_str!("../../fixtures/memory/fedora.txt");
    const NO_SWAP: &str = include_str!("../../fixtures/memory/noswap.txt");

    #[test]
    fn fedora_fixture_parses_sizes() {
        let snapshot = memory_snapshot(FEDORA).expect("parse");
        assert_eq!(snapshot.total_bytes, 16_020_068 * KIB);
        assert_eq!(snapshot.available_bytes, 2_876_004 * KIB);
        assert_eq!(snapshot.swap_used_bytes(), (8_388_604 - 2_255_772) * KIB);
        assert_eq!(snapshot.oom_kills, 3);
        assert!(snapshot.pressure.is_some());
    }

    #[test]
    fn used_pct_is_total_minus_available() {
        let snapshot = memory_snapshot(FEDORA).expect("parse");
        let expected = (16_020_068 - 2_876_004) as f64 * 100.0 / 16_020_068.0;
        assert!((snapshot.used_pct() - expected).abs() < 0.001);
    }

    #[test]
    fn noswap_fixture_has_zero_swap_and_no_pressure() {
        let snapshot = memory_snapshot(NO_SWAP).expect("parse");
        assert_eq!(snapshot.swap_total_bytes, 0);
        assert_eq!(snapshot.swap_used_pct(), 0.0);
        assert_eq!(snapshot.pressure, None);
    }

    #[test]
    fn missing_memtotal_is_parse_error() {
        assert!(matches!(
            memory_snapshot("###meminfo\nMemFree: 1 kB\n"),
            Err(ModuleError::Parse(_))
        ));
    }
}
