use chrono::{DateTime, Utc};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UnitOrigin {
    Vendor,
    Custom,
    Unknown,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Unit {
    pub name: String,
    pub load: String,
    pub active: String,
    pub sub: String,
    pub description: String,
    pub restarts: u32,
    pub main_pid: u32,
    pub active_since: Option<DateTime<Utc>>,
    pub fragment_path: String,
    pub working_directory: String,
    pub result: String,
}

impl Unit {
    pub fn is_failed(&self) -> bool {
        self.active == "failed"
    }

    pub fn is_active(&self) -> bool {
        self.active == "active"
    }

    pub fn origin(&self) -> UnitOrigin {
        if self.fragment_path.starts_with("/etc/") || self.fragment_path.starts_with("/usr/local/")
        {
            return UnitOrigin::Custom;
        }
        if self.fragment_path.starts_with("/usr/lib/") || self.fragment_path.starts_with("/lib/") {
            return UnitOrigin::Vendor;
        }
        UnitOrigin::Unknown
    }

    pub fn short_name(&self) -> &str {
        self.name.strip_suffix(".service").unwrap_or(&self.name)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Timer {
    pub name: String,
    pub activates: String,
    pub next: Option<DateTime<Utc>>,
    pub last: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ServicesSnapshot {
    pub units: Vec<Unit>,
    pub timers: Vec<Timer>,
}

impl ServicesSnapshot {
    pub fn failed(&self) -> impl Iterator<Item = &Unit> {
        self.units.iter().filter(|u| u.is_failed())
    }

    pub fn active_count(&self) -> usize {
        self.units.iter().filter(|u| u.is_active()).count()
    }

    pub fn custom(&self) -> impl Iterator<Item = &Unit> {
        self.units
            .iter()
            .filter(|u| u.origin() == UnitOrigin::Custom)
    }
}
