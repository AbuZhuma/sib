use asiba_core::{Alert, Incident, Severity};
use notify_rust::{Notification, Urgency};

const APP_NAME: &str = "Asiba";

pub fn send_desktop(alert: &Alert) {
    let summary = format!("{}: {}", alert.server, alert.rule_name);
    send(&summary, &alert.message, alert.severity);
}

pub fn send_desktop_incident(incident: &Incident) {
    let summary = format!("{}: {}", incident.server, incident.kind.key());
    send(&summary, &incident.summary, incident.severity);
}

fn urgency(severity: Severity) -> Urgency {
    match severity {
        Severity::Critical => Urgency::Critical,
        Severity::Warning => Urgency::Normal,
        Severity::Info => Urgency::Low,
    }
}

fn send(summary: &str, body: &str, severity: Severity) {
    let result = Notification::new()
        .appname(APP_NAME)
        .summary(summary)
        .body(body)
        .urgency(urgency(severity))
        .show();
    if let Err(error) = result {
        tracing::warn!(%error, "уведомление на рабочий стол не отправлено");
    }
}
