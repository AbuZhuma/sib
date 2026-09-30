use std::time::Duration;

#[derive(Debug, Clone, PartialEq)]
pub struct LoadAverage {
    pub one: f64,
    pub five: f64,
    pub fifteen: f64,
}

#[derive(Debug, Clone, PartialEq)]
pub struct SystemInfo {
    pub hostname: String,
    pub os_name: String,
    pub os_id: String,
    pub kernel: String,
    pub arch: String,
    pub uptime: Duration,
    pub load: LoadAverage,
    pub cpu_model: String,
    pub cpu_cores: u32,
    pub mem_total_bytes: u64,
    pub swap_total_bytes: u64,
    pub virtualization: Option<String>,
    pub timezone: Option<String>,
    pub clock_offset_secs: i64,
}

impl SystemInfo {
    pub fn uptime_human(&self) -> String {
        let secs = self.uptime.as_secs();
        let days = secs / 86_400;
        let hours = (secs % 86_400) / 3_600;
        let minutes = (secs % 3_600) / 60;
        if days > 0 {
            return format!("{days}d {hours}h {minutes}m");
        }
        if hours > 0 {
            return format!("{hours}h {minutes}m");
        }
        format!("{minutes}m")
    }
}
