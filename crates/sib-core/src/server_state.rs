use std::collections::BTreeMap;

use chrono::{DateTime, Utc};

use crate::event::Event;
use crate::module::{Availability, ModuleId};
use crate::series::{Point, Series};
use crate::server::{Location, ServerSpec};
use crate::snapshot::{ModuleData, Sample, Snapshot};

const MAX_SERVER_EVENTS: usize = 500;

#[derive(Debug, Clone)]
pub struct ServerState {
    pub spec: ServerSpec,
    pub connection: ConnectionStatus,
    pub modules: BTreeMap<ModuleId, ModuleState>,
    pub series: BTreeMap<String, Series>,
    pub ping: Option<PingStatus>,
    pub recent_events: Vec<Event>,
    pub location: Option<Location>,
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
            location: None,
        }
    }

    pub fn restarted(spec: ServerSpec, previous: Option<Self>) -> Self {
        let same_host = |p: &Self| p.spec.host == spec.host && p.spec.port == spec.port;
        match previous.filter(same_host) {
            Some(previous) => Self {
                spec,
                connection: ConnectionStatus::Connecting,
                ..previous
            },
            None => Self::new(spec),
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
    pub is_jump_host: bool,
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{AuthMethod, ServerDescription, ServerId, SudoMode};

    fn spec(host: &str) -> ServerSpec {
        ServerSpec {
            id: ServerId::parse("neo").expect("id"),
            host: host.into(),
            port: 22,
            user: "u".into(),
            auth: AuthMethod::Auto,
            jump: None,
            sudo: SudoMode::None,
            description: ServerDescription::default(),
            location: None,
            modules: Default::default(),
            checks: Vec::new(),
            check_overrides: Default::default(),
            pipelines: Vec::new(),
        }
    }

    #[test]
    fn restarted_keeps_series_for_same_host_and_resets_for_new_host() {
        let mut previous = ServerState::new(spec("a"));
        previous.push_samples(Utc::now(), &[Sample::new("cpu.total", 1.0)]);
        let kept = ServerState::restarted(spec("a"), Some(previous.clone()));
        assert_eq!(kept.series.len(), 1);
        assert_eq!(kept.connection, ConnectionStatus::Connecting);
        let reset = ServerState::restarted(spec("b"), Some(previous));
        assert!(reset.series.is_empty());
    }
}
