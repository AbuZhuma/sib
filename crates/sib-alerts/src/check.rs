use chrono::{DateTime, Utc};
use sib_core::{Alert, ConnectionStatus, METRIC_OFFLINE, ServerId, ServerState, Severity};

use crate::baseline::Deviation;

const BASELINE_FOR_SECS: u64 = 60;
const BASELINE_RULE_PREFIX: &str = "baseline:";
const MAX_VALUE_AGE: chrono::Duration = chrono::Duration::hours(1);

pub struct Check {
    pub rule_id: String,
    pub rule_name: String,
    pub metric: String,
    pub severity: Severity,
    pub for_secs: u64,
    pub value: f64,
    pub holds: bool,
    pub message: String,
}

impl Check {
    pub fn into_alert(self, id: u64, server: ServerId, since: DateTime<Utc>) -> Alert {
        Alert {
            id,
            server,
            rule_id: self.rule_id,
            rule_name: self.rule_name,
            metric: self.metric,
            severity: self.severity,
            value: self.value,
            message: self.message,
            started_at: since,
            resolved_at: None,
            acknowledged: false,
            muted_until: None,
        }
    }
}

pub fn baseline_check(metric: &str, value: f64, deviation: Option<Deviation>) -> Check {
    let message = deviation
        .map(|d| {
            format!(
                "{metric} = {value:.1}, базовая линия {:.1} ({:+.1}σ)",
                d.mean, d.sigmas
            )
        })
        .unwrap_or_default();
    Check {
        rule_id: format!("{BASELINE_RULE_PREFIX}{metric}"),
        rule_name: format!("Отклонение {metric}"),
        metric: metric.to_owned(),
        severity: Severity::Warning,
        for_secs: BASELINE_FOR_SECS,
        value,
        holds: deviation.is_some(),
        message,
    }
}

pub fn metric_value(server: &ServerState, metric: &str, now: DateTime<Utc>) -> Option<f64> {
    if metric == METRIC_OFFLINE {
        return Some(match &server.connection {
            ConnectionStatus::Offline { .. } => 1.0,
            _ => 0.0,
        });
    }
    if !server.connection.is_online() || is_module_failing(server, metric) {
        return None;
    }
    let point = server.series.get(metric)?.latest()?;
    (now - point.at <= MAX_VALUE_AGE).then_some(point.value)
}

fn is_module_failing(server: &ServerState, metric: &str) -> bool {
    let module = metric.split('.').next().unwrap_or_default();
    server
        .modules
        .iter()
        .any(|(id, state)| id.0 == module && state.last_error.is_some())
}
