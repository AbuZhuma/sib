use std::collections::BTreeMap;
use std::sync::{Arc, RwLock};

use chrono::{DateTime, Utc};

use crate::event::Event;
use crate::module::{Availability, ModuleId};
use crate::series::{Point, Series};
use crate::server::{ServerId, ServerSpec};
use crate::snapshot::{ModuleData, Sample, Snapshot};

const MAX_EVENTS: usize = 500;

pub type SharedState = Arc<RwLock<AppState>>;

#[derive(Debug, Default)]
pub struct AppState {
    pub servers: BTreeMap<ServerId, ServerState>,
    pub events: Vec<Event>,
}

impl AppState {
    pub fn shared() -> SharedState {
        Arc::new(RwLock::new(Self::default()))
    }

    pub fn push_events(&mut self, events: impl IntoIterator<Item = Event>) {
        self.events.extend(events);
        if self.events.len() > MAX_EVENTS {
            let overflow = self.events.len() - MAX_EVENTS;
            self.events.drain(..overflow);
        }
    }

    pub fn online_count(&self) -> usize {
        self.servers
            .values()
            .filter(|s| s.connection.is_online())
            .count()
    }
}

#[derive(Debug, Clone)]
pub struct ServerState {
    pub spec: ServerSpec,
    pub connection: ConnectionStatus,
    pub modules: BTreeMap<ModuleId, ModuleState>,
    pub series: BTreeMap<String, Series>,
    pub ping: Option<PingStatus>,
}

impl ServerState {
    pub fn new(spec: ServerSpec) -> Self {
        Self {
            spec,
            connection: ConnectionStatus::Connecting,
            modules: BTreeMap::new(),
            series: BTreeMap::new(),
            ping: None,
        }
    }

    pub fn snapshot(&self, module: ModuleId) -> Option<&Snapshot> {
        self.modules.get(&module)?.last_snapshot.as_ref()
    }

    pub fn data<T: ModuleData>(&self, module: ModuleId) -> Option<&T> {
        self.snapshot(module)?.downcast::<T>()
    }

    pub fn push_samples(&mut self, at: DateTime<Utc>, samples: &[Sample]) {
        for sample in samples {
            let series = self.series.entry(sample.key.clone()).or_default();
            series.push(Point {
                at,
                value: sample.value,
            });
        }
    }

    pub fn latest_value(&self, key: &str) -> Option<f64> {
        self.series.get(key)?.latest().map(|p| p.value)
    }

    pub fn available_modules(&self) -> impl Iterator<Item = ModuleId> + '_ {
        self.modules
            .iter()
            .filter(|(_, state)| state.availability.is_usable())
            .map(|(id, _)| *id)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct PingStatus {
    pub rtt_ms: Option<f64>,
    pub at: DateTime<Utc>,
    pub lost_in_row: u32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ConnectionStatus {
    Connecting,
    Online {
        since: DateTime<Utc>,
    },
    Offline {
        reason: String,
        retry_at: DateTime<Utc>,
    },
    UntrustedHostKey {
        fingerprint: String,
        changed: bool,
    },
}

impl ConnectionStatus {
    pub fn is_online(&self) -> bool {
        matches!(self, Self::Online { .. })
    }
}

#[derive(Debug, Clone)]
pub struct ModuleState {
    pub availability: Availability,
    pub last_snapshot: Option<Snapshot>,
    pub last_error: Option<String>,
    pub last_collected: Option<DateTime<Utc>>,
}

impl ModuleState {
    pub fn detected(availability: Availability) -> Self {
        Self {
            availability,
            last_snapshot: None,
            last_error: None,
            last_collected: None,
        }
    }

    pub fn record_snapshot(&mut self, snapshot: Snapshot) {
        self.last_collected = Some(snapshot.taken_at);
        self.last_snapshot = Some(snapshot);
        self.last_error = None;
    }

    pub fn record_error(&mut self, error: impl ToString) {
        self.last_error = Some(error.to_string());
    }
}
