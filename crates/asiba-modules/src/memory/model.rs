use crate::common::pressure::Pressure;
use crate::common::rate;

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

    pub fn swap_used_bytes(&self) -> u64 {
        self.swap_total_bytes.saturating_sub(self.swap_free_bytes)
    }

    pub fn swap_used_pct(&self) -> f64 {
        rate::percent(self.swap_used_bytes(), self.swap_total_bytes)
    }
}
