use std::collections::BTreeMap;
use std::sync::{Arc, RwLock};

use crate::action::ActionRecord;
use crate::alert::Alert;
use crate::audit::{AuditReport, AuditScope, AuditTarget};
use crate::event::Event;
use crate::incident::{IgnoredIncident, Incident, IncidentKind};
use crate::server::{Location, ServerId};
use crate::server_state::ServerState;

const MAX_EVENTS: usize = 500;
const MAX_ACTIONS: usize = 200;
const MAX_RESOLVED_ALERTS: usize = 300;
const MAX_RESOLVED_INCIDENTS: usize = 300;
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

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;

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
}
