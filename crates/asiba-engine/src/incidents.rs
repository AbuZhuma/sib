use asiba_core::{AppState, Event, Incident, ModuleId};
use chrono::{DateTime, Utc};

pub const MODULE: ModuleId = ModuleId("incident");

pub fn reconcile(state: &mut AppState, now: DateTime<Utc>) -> Vec<Incident> {
    let reconciled = asiba_incidents::reconcile(state, now);
    let events: Vec<Event> = reconciled
        .opened
        .iter()
        .map(|incident| {
            Event::new(MODULE, incident.severity, incident.summary.clone())
                .for_server(incident.server.clone())
        })
        .collect();
    for event in &events {
        if let Some(server) = event
            .server
            .as_ref()
            .and_then(|id| state.servers.get_mut(id))
        {
            server.push_recent_events([event.clone()]);
        }
    }
    state.push_events(events);
    reconciled.opened
}

pub fn announce(opened: &[Incident], notify_desktop: bool) {
    for incident in opened {
        tracing::info!(
            server = %incident.server,
            kind = incident.kind.key(),
            "инцидент: {}",
            incident.summary
        );
        if notify_desktop && incident.kind != asiba_core::IncidentKind::Alert {
            asiba_alerts::send_desktop_incident(incident);
        }
    }
}
