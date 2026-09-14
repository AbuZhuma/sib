use asiba_core::{Alert, Severity};
use notify_rust::{Notification, Urgency};

const APP_NAME: &str = "Asiba";

pub fn send_desktop(alert: &Alert) {
    let urgency = match alert.severity {
        Severity::Critical => Urgency::Critical,
        Severity::Warning => Urgency::Normal,
        Severity::Info => Urgency::Low,
    };
    let result = Notification::new()
        .appname(APP_NAME)
        .summary(&format!("{}: {}", alert.server, alert.rule_name))
        .body(&alert.message)
        .urgency(urgency)
        .show();
    if let Err(error) = result {
        tracing::warn!(%error, "уведомление на рабочий стол не отправлено");
    }
}
