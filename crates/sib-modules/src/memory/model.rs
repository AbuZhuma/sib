use chrono::{DateTime, Duration, Utc};

use crate::common::pressure::Pressure;
use crate::common::rate;

pub const OOM_RECENT_WINDOW: Duration = Duration::hours(24);

#[derive(Debug, Clone, PartialEq)]
pub struct MemorySnapshot {
    pub total_bytes: u64,
    pub free_bytes: u64,
    pub available_bytes: u64,
    pub buffers_bytes: u64,
    pub cached_bytes: u64,
    pub shmem_bytes: u64,
    pub reclaimable_bytes: u64,
    pub dirty_bytes: u64,
    pub swap_total_bytes: u64,
    pub swap_free_bytes: u64,
    pub pressure: Option<Pressure>,
    pub oom_kills: u64,
    pub oom_kills_observed: u64,
    pub last_oom_at: Option<DateTime<Utc>>,
    pub swap_in_pages: u64,
    pub swap_out_pages: u64,
    pub swap_in_ps: Option<f64>,
    pub swap_out_ps: Option<f64>,
}

impl MemorySnapshot {
    pub fn used_bytes(&self) -> u64 {
        self.total_bytes.saturating_sub(self.available_bytes)
    }

    pub fn used_pct(&self) -> f64 {
        rate::percent(self.used_bytes(), self.total_bytes)
    }

    pub fn recent_oom_kills(&self, now: DateTime<Utc>) -> u64 {
        match self.last_oom_at {
            Some(at) if now - at <= OOM_RECENT_WINDOW => self.oom_kills_observed,
            _ => 0,
        }
    }

    pub fn swap_used_bytes(&self) -> u64 {
        self.swap_total_bytes.saturating_sub(self.swap_free_bytes)
    }

    pub fn swap_used_pct(&self) -> f64 {
        rate::percent(self.swap_used_bytes(), self.swap_total_bytes)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn snapshot(last_oom_at: Option<DateTime<Utc>>) -> MemorySnapshot {
        MemorySnapshot {
            total_bytes: 100,
            free_bytes: 0,
            available_bytes: 50,
            buffers_bytes: 0,
            cached_bytes: 0,
            shmem_bytes: 0,
            reclaimable_bytes: 0,
            dirty_bytes: 0,
            swap_total_bytes: 0,
            swap_free_bytes: 0,
            pressure: None,
            oom_kills: 7,
            oom_kills_observed: 2,
            last_oom_at,
            swap_in_pages: 0,
            swap_out_pages: 0,
            swap_in_ps: None,
            swap_out_ps: None,
        }
    }

    #[test]
    fn recent_oom_kills_counts_only_kills_seen_within_window() {
        let now = Utc::now();
        assert_eq!(snapshot(None).recent_oom_kills(now), 0);
        assert_eq!(
            snapshot(Some(now - Duration::hours(1))).recent_oom_kills(now),
            2
        );
        assert_eq!(
            snapshot(Some(now - Duration::hours(30))).recent_oom_kills(now),
            0
        );
    }
}
