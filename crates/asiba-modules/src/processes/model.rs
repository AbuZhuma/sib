use std::collections::HashMap;

use crate::common::rate;

#[derive(Debug, Clone, PartialEq)]
pub struct Process {
    pub pid: u32,
    pub ppid: u32,
    pub user: String,
    pub comm: String,
    pub cmdline: String,
    pub state: char,
    pub threads: u32,
    pub cpu_ticks: u64,
    pub start_ticks: u64,
    pub rss_bytes: u64,
    pub vsz_bytes: u64,
    pub read_bytes: Option<u64>,
    pub write_bytes: Option<u64>,
    pub cpu_pct: Option<f64>,
    pub read_bps: Option<f64>,
    pub write_bps: Option<f64>,
}

impl Process {
    pub fn is_zombie(&self) -> bool {
        self.state == 'Z'
    }

    pub fn is_running(&self) -> bool {
        self.state == 'R'
    }

    pub fn display_name(&self) -> &str {
        if self.cmdline.is_empty() {
            &self.comm
        } else {
            &self.cmdline
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct ProcessSnapshot {
    pub processes: Vec<Process>,
    pub clock_ticks_per_sec: u64,
    pub uptime_secs: f64,
    pub total_memory_bytes: u64,
}

impl ProcessSnapshot {
    pub fn running_count(&self) -> usize {
        self.processes.iter().filter(|p| p.is_running()).count()
    }

    pub fn zombie_count(&self) -> usize {
        self.processes.iter().filter(|p| p.is_zombie()).count()
    }

    pub fn memory_pct(&self, process: &Process) -> f64 {
        rate::percent(process.rss_bytes, self.total_memory_bytes)
    }

    pub fn started_secs_ago(&self, process: &Process) -> f64 {
        let started = process.start_ticks as f64 / self.clock_ticks_per_sec.max(1) as f64;
        (self.uptime_secs - started).max(0.0)
    }
}

pub fn fill_rates(current: &mut ProcessSnapshot, previous: &ProcessSnapshot, elapsed: f64) {
    let hz = current.clock_ticks_per_sec.max(1) as f64;
    let earlier: HashMap<(u32, u64), &Process> = previous
        .processes
        .iter()
        .map(|p| ((p.pid, p.start_ticks), p))
        .collect();
    for process in &mut current.processes {
        let Some(before) = earlier.get(&(process.pid, process.start_ticks)) else {
            continue;
        };
        let ticks = rate::per_second(process.cpu_ticks, before.cpu_ticks, elapsed);
        process.cpu_pct = Some(ticks / hz * 100.0);
        process.read_bps = rate_between(process.read_bytes, before.read_bytes, elapsed);
        process.write_bps = rate_between(process.write_bytes, before.write_bytes, elapsed);
    }
}

fn rate_between(current: Option<u64>, previous: Option<u64>, elapsed: f64) -> Option<f64> {
    Some(rate::per_second(current?, previous?, elapsed))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn process(pid: u32, cpu_ticks: u64) -> Process {
        Process {
            pid,
            ppid: 1,
            user: "root".into(),
            comm: "x".into(),
            cmdline: String::new(),
            state: 'S',
            threads: 1,
            cpu_ticks,
            start_ticks: 100,
            rss_bytes: 0,
            vsz_bytes: 0,
            read_bytes: Some(0),
            write_bytes: None,
            cpu_pct: None,
            read_bps: None,
            write_bps: None,
        }
    }

    fn snapshot(processes: Vec<Process>) -> ProcessSnapshot {
        ProcessSnapshot {
            processes,
            clock_ticks_per_sec: 100,
            uptime_secs: 1000.0,
            total_memory_bytes: 1000,
        }
    }

    #[test]
    fn fill_rates_computes_cpu_percent_from_ticks() {
        let previous = snapshot(vec![process(7, 100)]);
        let mut current = snapshot(vec![process(7, 200)]);
        fill_rates(&mut current, &previous, 2.0);
        assert_eq!(current.processes[0].cpu_pct, Some(50.0));
        assert_eq!(current.processes[0].read_bps, Some(0.0));
        assert_eq!(current.processes[0].write_bps, None);
    }

    #[test]
    fn fill_rates_skips_new_pid() {
        let previous = snapshot(vec![]);
        let mut current = snapshot(vec![process(7, 200)]);
        fill_rates(&mut current, &previous, 2.0);
        assert_eq!(current.processes[0].cpu_pct, None);
    }
}
