use crate::common::pressure::Pressure;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct CpuTimes {
    pub user: u64,
    pub nice: u64,
    pub system: u64,
    pub idle: u64,
    pub iowait: u64,
    pub irq: u64,
    pub softirq: u64,
    pub steal: u64,
}

impl CpuTimes {
    pub fn total(&self) -> u64 {
        self.user
            + self.nice
            + self.system
            + self.idle
            + self.iowait
            + self.irq
            + self.softirq
            + self.steal
    }

    pub fn idle_total(&self) -> u64 {
        self.idle + self.iowait
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct CoreUsage {
    pub busy: f64,
    pub user: f64,
    pub system: f64,
    pub iowait: f64,
    pub steal: f64,
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct CpuUsage {
    pub total: CoreUsage,
    pub cores: Vec<CoreUsage>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct CpuSnapshot {
    pub times: Vec<CpuTimes>,
    pub usage: Option<CpuUsage>,
    pub pressure: Option<Pressure>,
    pub frequency_mhz: Option<f64>,
    pub temperature_c: Option<f64>,
}

impl CpuSnapshot {
    pub fn core_count(&self) -> usize {
        self.times.len().saturating_sub(1)
    }
}

pub fn usage_between(previous: &[CpuTimes], current: &[CpuTimes]) -> Option<CpuUsage> {
    if previous.len() != current.len() || current.is_empty() {
        return None;
    }
    let mut usages = previous.iter().zip(current).map(|(p, c)| core_usage(p, c));
    let total = usages.next()?;
    Some(CpuUsage {
        total,
        cores: usages.collect(),
    })
}

fn core_usage(previous: &CpuTimes, current: &CpuTimes) -> CoreUsage {
    let total = current.total().saturating_sub(previous.total());
    if total == 0 {
        return CoreUsage::default();
    }
    let share = |a: u64, b: u64| a.saturating_sub(b) as f64 * 100.0 / total as f64;
    let idle = share(current.idle_total(), previous.idle_total());
    CoreUsage {
        busy: (100.0 - idle).clamp(0.0, 100.0),
        user: share(current.user + current.nice, previous.user + previous.nice),
        system: share(
            current.system + current.irq + current.softirq,
            previous.system + previous.irq + previous.softirq,
        ),
        iowait: share(current.iowait, previous.iowait),
        steal: share(current.steal, previous.steal),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn usage_between_half_busy_is_fifty_percent() {
        let previous = [CpuTimes::default()];
        let current = [CpuTimes {
            user: 50,
            idle: 50,
            ..Default::default()
        }];
        let usage = usage_between(&previous, &current).expect("usage");
        assert_eq!(usage.total.busy, 50.0);
        assert_eq!(usage.total.user, 50.0);
    }

    #[test]
    fn usage_between_mismatched_core_count_is_none() {
        assert_eq!(usage_between(&[CpuTimes::default()], &[]), None);
    }
}
