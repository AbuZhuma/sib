use std::collections::BTreeMap;
use std::sync::{Arc, RwLock};

use chrono::{DateTime, Utc};

use crate::action::ActionRecord;
use crate::alert::Alert;
use crate::audit::{AuditReport, AuditScope, AuditTarget};
use crate::event::Event;
use crate::incident::{IgnoredIncident, Incident, IncidentKind};
use crate::module::{Availability, ModuleId};
use crate::series::{Point, Series};
use crate::server::{Location, ServerId, ServerSpec};
use crate::snapshot::{ModuleData, Sample, Snapshot};

const MAX_EVENTS: usize = 500;
const MAX_ACTIONS: usize = 200;
const MAX_RESOLVED_ALERTS: usize = 300;
const MAX_RESOLVED_INCIDENTS: usize = 300;
const MAX_SERVER_EVENTS: usize = 500;
const MAX_AUDITS: usize = 200;

pub type SharedState = Arc<RwLock<AppState>>;

#[derive(Debug, Default, Clone)]
pub struct AppState {
    pub servers: BTreeMap<ServerId, ServerState>,
    pub events: Vec<Event>,
    pub actions: Vec<ActionRecord>,
    pub alerts: Vec<Alert>,
    pub incidents: Vec<Incident>,
    pub ignored_incidents: Vec<IgnoredIncident>,
    pub audits: Vec<AuditReport>,
    pub self_location: Option<Location>,
    pub ip_countries: BTreeMap<String, String>,
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

    pub fn country_of(&self, ip: &str) -> Option<&str> {
        self.ip_countries.get(ip).map(String::as_str)
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

    pub fn audit_mut(&mut self, id: u64) -> Option<&mut AuditReport> {
        self.audits.iter_mut().find(|a| a.id == id)
    }

    pub fn push_audit(&mut self, report: AuditReport) {
        self.audits.push(report);
        let finished = self.audits.iter().filter(|a| !a.is_running()).count();
        if finished <= MAX_AUDITS {
            return;
        }
        let mut overflow = finished - MAX_AUDITS;
        self.audits.retain(|a| {
            if a.is_running() || overflow == 0 {
                return true;
            }
            overflow -= 1;
            false
        });
    }

    pub fn latest_audit(&self, target: &AuditTarget, scope: &AuditScope) -> Option<&AuditReport> {
        self.audits
            .iter()
            .rev()
            .find(|a| &a.target == target && &a.scope == scope)
    }

    pub fn is_incident_ignored(
        &self,
        server: &ServerId,
        kind: IncidentKind,
        subject: &str,
    ) -> bool {
        self.ignored_incidents
            .iter()
            .any(|i| i.matches(server, kind, subject))
    }

    pub fn active_incidents(&self) -> impl Iterator<Item = &Incident> {
        self.incidents.iter().filter(|i| i.is_active())
    }

    pub fn trim_resolved_incidents(&mut self) {
        let resolved = self.incidents.iter().filter(|i| !i.is_active()).count();
        if resolved <= MAX_RESOLVED_INCIDENTS {
            return;
        }
        let mut overflow = resolved - MAX_RESOLVED_INCIDENTS;
        self.incidents.retain(|i| {
            if i.is_active() || overflow == 0 {
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{AuthMethod, ServerDescription, SudoMode};

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
        }
    }

    #[test]
    fn push_audit_drops_oldest_finished_reports_beyond_cap() {
        let mut state = AppState::default();
        let report = |id: u64, status: crate::AuditStatus| AuditReport {
            id,
            target: AuditTarget::Fleet,
            scope: AuditScope::Full,
            started_at: Utc::now(),
            finished_at: None,
            model: String::new(),
            context_tokens: 0,
            text: String::new(),
            status,
        };
        state.push_audit(report(0, crate::AuditStatus::Running));
        for id in 1..=MAX_AUDITS as u64 + 1 {
            state.push_audit(report(id, crate::AuditStatus::Done));
        }
        assert_eq!(state.audits.len(), MAX_AUDITS + 1);
        assert!(state.audits.iter().any(|a| a.id == 0));
        assert!(!state.audits.iter().any(|a| a.id == 1));
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
