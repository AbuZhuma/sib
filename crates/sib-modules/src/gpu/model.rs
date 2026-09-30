#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Vendor {
    Nvidia,
    Amd,
}

impl Vendor {
    pub fn label(self) -> &'static str {
        match self {
            Self::Nvidia => "NVIDIA",
            Self::Amd => "AMD",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GpuProcess {
    pub pid: u32,
    pub name: String,
    pub memory_bytes: u64,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Gpu {
    pub index: u32,
    pub name: String,
    pub vendor: Vendor,
    pub utilization_pct: f64,
    pub memory_used: u64,
    pub memory_total: u64,
    pub temperature_c: Option<f64>,
    pub power_w: Option<f64>,
    pub power_limit_w: Option<f64>,
    pub processes: Vec<GpuProcess>,
}

impl Gpu {
    pub fn memory_pct(&self) -> f64 {
        if self.memory_total == 0 {
            return 0.0;
        }
        self.memory_used as f64 * 100.0 / self.memory_total as f64
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct GpuSnapshot {
    pub gpus: Vec<Gpu>,
}

impl GpuSnapshot {
    pub fn max_utilization(&self) -> f64 {
        self.gpus
            .iter()
            .map(|g| g.utilization_pct)
            .fold(0.0, f64::max)
    }
}
