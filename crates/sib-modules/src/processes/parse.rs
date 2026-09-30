use std::collections::HashMap;

use sib_core::ModuleError;

use super::model::{Process, ProcessSnapshot};
use crate::common::sections::Sections;

const DEFAULT_HZ: u64 = 100;
const DEFAULT_PAGE_SIZE: u64 = 4096;
const KIB: u64 = 1024;

pub fn process_snapshot(raw: &str) -> Result<ProcessSnapshot, ModuleError> {
    let sections = Sections::parse(raw);
    let page_size = number(sections.get_or_empty("pagesize")).unwrap_or(DEFAULT_PAGE_SIZE);
    let users = pid_map(sections.get_or_empty("users"), ' ');
    let cmdlines = pid_map(sections.get_or_empty("cmd"), '\t');
    let io = pid_map(sections.get_or_empty("io"), '\t');
    let mut processes: Vec<Process> = sections
        .get_or_empty("stat")
        .lines()
        .filter_map(|line| stat_line(line, page_size))
        .collect();
    if processes.is_empty() {
        return Err(ModuleError::Parse("нет строк /proc/[pid]/stat".to_owned()));
    }
    for process in &mut processes {
        let pid = process.pid;
        process.user = users
            .get(&pid)
            .map(|u| u.trim().to_owned())
            .unwrap_or_default();
        process.cmdline = cmdlines
            .get(&pid)
            .map(|c| c.trim().to_owned())
            .unwrap_or_default();
        if let Some(io_line) = io.get(&pid) {
            process.read_bytes = io_field(io_line, "read_bytes");
            process.write_bytes = io_field(io_line, "write_bytes");
        }
    }
    Ok(ProcessSnapshot {
        processes,
        clock_ticks_per_sec: number(sections.get_or_empty("hz")).unwrap_or(DEFAULT_HZ),
        uptime_secs: uptime(sections.get_or_empty("uptime")),
        total_memory_bytes: memtotal(sections.get_or_empty("memtotal")),
    })
}

fn stat_line(line: &str, page_size: u64) -> Option<Process> {
    let (pid, rest) = line.split_once(" (")?;
    let (comm, rest) = rest.rsplit_once(") ")?;
    let fields: Vec<&str> = rest.split_whitespace().collect();
    if fields.len() < 22 {
        return None;
    }
    let field = |index: usize| fields[index].parse::<u64>().ok();
    Some(Process {
        pid: pid.parse().ok()?,
        ppid: fields[1].parse().ok()?,
        user: String::new(),
        comm: comm.to_owned(),
        cmdline: String::new(),
        state: fields[0].chars().next()?,
        threads: fields[17].parse().unwrap_or(1),
        cpu_ticks: field(11)? + field(12)?,
        start_ticks: field(19)?,
        rss_bytes: field(21)? * page_size,
        vsz_bytes: field(20)?,
        read_bytes: None,
        write_bytes: None,
        cpu_pct: None,
        read_bps: None,
        write_bps: None,
    })
}

fn pid_map(raw: &str, separator: char) -> HashMap<u32, &str> {
    raw.lines()
        .filter_map(|line| {
            let trimmed = line.trim_start();
            let (pid, rest) = trimmed.split_once(separator)?;
            Some((pid.trim().parse().ok()?, rest))
        })
        .collect()
}

fn io_field(line: &str, key: &str) -> Option<u64> {
    let mut parts = line.split_whitespace();
    while let Some(part) = parts.next() {
        if part.strip_suffix(':') == Some(key) {
            return parts.next()?.parse().ok();
        }
    }
    None
}

fn number(raw: &str) -> Option<u64> {
    raw.trim().parse().ok()
}

fn uptime(raw: &str) -> f64 {
    raw.split_whitespace()
        .next()
        .and_then(|v| v.parse().ok())
        .unwrap_or(0.0)
}

fn memtotal(raw: &str) -> u64 {
    raw.split_whitespace()
        .nth(1)
        .and_then(|v| v.parse::<u64>().ok())
        .unwrap_or(0)
        * KIB
}

#[cfg(test)]
mod tests {
    use super::*;

    const FEDORA: &str = include_str!("../../fixtures/processes/fedora.txt");

    #[test]
    fn fedora_fixture_parses_processes() {
        let snapshot = process_snapshot(FEDORA).expect("parse");
        assert_eq!(snapshot.processes.len(), 3);
        let boltd = snapshot
            .processes
            .iter()
            .find(|p| p.pid == 1001)
            .expect("boltd");
        assert_eq!(boltd.comm, "boltd");
        assert_eq!(boltd.user, "root");
        assert_eq!(boltd.cpu_ticks, 28);
        assert_eq!(boltd.threads, 4);
        assert_eq!(boltd.rss_bytes, 1520 * 4096);
        assert_eq!(boltd.read_bytes, Some(12288));
        assert_eq!(boltd.cmdline, "/usr/libexec/boltd");
        assert_eq!(snapshot.clock_ticks_per_sec, 100);
        assert_eq!(snapshot.total_memory_bytes, 16020068 * KIB);
    }

    #[test]
    fn comm_with_spaces_and_parens_is_kept() {
        let snapshot = process_snapshot(FEDORA).expect("parse");
        let odd = snapshot
            .processes
            .iter()
            .find(|p| p.pid == 42)
            .expect("odd");
        assert_eq!(odd.comm, "my (odd) name");
        assert!(odd.is_zombie());
    }

    #[test]
    fn empty_stat_is_parse_error() {
        assert!(matches!(
            process_snapshot("###stat\n"),
            Err(ModuleError::Parse(_))
        ));
    }
}
