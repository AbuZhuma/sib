use std::collections::BTreeMap;
use std::sync::{Arc, RwLock};

use chrono::{DateTime, Utc};

use crate::action::ActionRecord;
use crate::alert::Alert;
use crate::event::Event;
use crate::module::{Availability, ModuleId};
use crate::series::{Point, Series};
use crate::server::{ServerId, ServerSpec};
use crate::snapshot::{ModuleData, Sample, Snapshot};

const MAX_EVENTS: usize = 500;
const MAX_ACTIONS: usize = 200;
const MAX_RESOLVED_ALERTS: usize = 300;
const MAX_SERVER_EVENTS: usize = 100;

pub type SharedState = Arc<RwLock<AppState>>;

#[derive(Debug, Default)]
pub struct AppState {
    pub servers: BTreeMap<ServerId, ServerState>,
    pub events: Vec<Event>,
    pub actions: Vec<ActionRecord>,
    pub alerts: Vec<Alert>,
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

    pub fn push_action(&mut self, record: ActionRecord) {
        self.actions.insert(0, record);
        self.actions.truncate(MAX_ACTIONS);
    }

    pub fn active_alerts(&self) -> impl Iterator<Item = &Alert> {
        self.alerts.iter().filter(|a| a.is_active())
    }

    pub fn alert_mut(&mut self, id: u64) -> Option<&mut Alert> {
        self.alerts.iter_mut().find(|a| a.id == id)
    }

    pub fn trim_resolved_alerts(&mut self) {
        let resolved = self.alerts.iter().filter(|a| !a.is_active()).count();
        if resolved <= MAX_RESOLVED_ALERTS {
            return;
        }
        let mut overflow = resolved - MAX_RESOLVED_ALERTS;
        self.alerts.retain(|a| {
            if a.is_active() || overflow == 0 {
                return true;
            }
            overflow -= 1;
            false
        });
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
    pub recent_events: Vec<Event>,
}

impl ServerState {
    pub fn new(spec: ServerSpec) -> Self {
        Self {
            spec,
            connection: ConnectionStatus::Connecting,
            modules: BTreeMap::new(),
            series: BTreeMap::new(),
            ping: None,
            recent_events: Vec::new(),
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

    pub fn push_recent_events(&mut self, events: impl IntoIterator<Item = Event>) {
        self.recent_events.extend(events);
        if self.recent_events.len() > MAX_SERVER_EVENTS {
            let overflow = self.recent_events.len() - MAX_SERVER_EVENTS;
            self.recent_events.drain(..overflow);
        }
    }

    pub fn prepend_history(&mut self, history: BTreeMap<String, Vec<Point>>) {
        for (key, points) in history {
            self.series.entry(key).or_default().prepend_history(&points);
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
