use std::collections::HashMap;

use asiba_core::{
    Alert, AlertRule, AppState, ConnectionStatus, METRIC_OFFLINE, ServerId, ServerState, Severity,
};
use asiba_modules::{anomalies, cpu, logs, network, processes, security};
use chrono::{DateTime, Utc};

use crate::baseline::{Baseline, Deviation};

const BASELINE_METRICS: [&str; 8] = [
    cpu::KEY_TOTAL,
    network::KEY_RX_BPS,
    network::KEY_TX_BPS,
    network::KEY_ESTABLISHED,
    processes::KEY_COUNT,
    logs::KEY_ERRORS_PER_MIN,
    security::KEY_FAILED_LOGINS,
    anomalies::KEY_PPS_IN,
];
const BASELINE_FOR_SECS: u64 = 60;
const BASELINE_RULE_PREFIX: &str = "baseline:";

type Key = (ServerId, String);

pub struct Raised(pub Vec<Alert>);

#[derive(Default)]
pub struct Evaluator {
    rules: Vec<AlertRule>,
    pending: HashMap<Key, DateTime<Utc>>,
    baselines: HashMap<Key, Baseline>,
    observed_at: HashMap<Key, DateTime<Utc>>,
    next_id: u64,
}

impl Evaluator {
    pub fn new(rules: Vec<AlertRule>) -> Self {
        Self {
            rules,
            next_id: 1,
            ..Self::default()
        }
    }

    pub fn set_rules(&mut self, rules: Vec<AlertRule>) {
        self.rules = rules;
        self.pending.clear();
    }

    pub fn evaluate(&mut self, state: &mut AppState, now: DateTime<Utc>) -> Raised {
        let mut raised = Vec::new();
        let servers: Vec<ServerId> = state.servers.keys().cloned().collect();
        for server_id in servers {
            let Some(server) = state.servers.get(&server_id) else {
                continue;
            };
            let mut checks = self.rule_checks(server);
            checks.extend(self.baseline_checks(server));
            for check in checks {
                if let Some(alert) = self.settle(state, &server_id, check, now) {
                    raised.push(alert);
                }
            }
        }
        state.trim_resolved_alerts();
        Raised(raised)
    }

    fn rule_checks(&self, server: &ServerState) -> Vec<Check> {
        self.rules
            .iter()
            .filter_map(|rule| {
                let value = metric_value(server, &rule.metric)?;
                Some(Check {
                    rule_id: rule.id.clone(),
                    rule_name: rule.name.clone(),
                    severity: rule.severity,
                    for_secs: rule.for_secs,
                    value,
                    holds: rule.condition.holds(value, rule.threshold),
                    message: format!(
                        "{} = {value:.1} ({} {})",
                        rule.metric,
                        rule.condition.symbol(),
                        rule.threshold
                    ),
                })
            })
            .collect()
    }

    fn baseline_checks(&mut self, server: &ServerState) -> Vec<Check> {
        let mut checks = Vec::new();
        for metric in BASELINE_METRICS {
            let key = (server.spec.id.clone(), metric.to_owned());
            let Some(point) = server.series.get(metric).and_then(|s| s.latest()) else {
                continue;
            };
            if self.observed_at.get(&key) == Some(&point.at) {
                continue;
            }
            self.observed_at.insert(key.clone(), point.at);
            let deviation = self.baselines.entry(key).or_default().observe(point.value);
            checks.push(baseline_check(metric, point.value, deviation));
        }
        checks
    }

    fn settle(
        &mut self,
        state: &mut AppState,
        server: &ServerId,
        check: Check,
        now: DateTime<Utc>,
    ) -> Option<Alert> {
        let key = (server.clone(), check.rule_id.clone());
        let active = state
            .alerts
            .iter_mut()
            .find(|a| a.is_active() && a.server == *server && a.rule_id == check.rule_id);
        if !check.holds {
            self.pending.remove(&key);
            if let Some(alert) = active {
                alert.resolved_at = Some(now);
            }
            return None;
        }
        if let Some(alert) = active {
            alert.value = check.value;
            alert.message = check.message;
            return None;
        }
        let since = *self.pending.entry(key).or_insert(now);
        if (now - since).num_seconds() < check.for_secs as i64 {
            return None;
        }
        let alert = Alert {
            id: self.next_id,
            server: server.clone(),
            rule_id: check.rule_id,
            rule_name: check.rule_name,
            severity: check.severity,
            value: check.value,
            message: check.message,
            started_at: since,
            resolved_at: None,
            acknowledged: false,
            muted_until: None,
        };
        self.next_id += 1;
        state.alerts.insert(0, alert.clone());
        Some(alert)
    }
}

struct Check {
    rule_id: String,
    rule_name: String,
    severity: Severity,
    for_secs: u64,
    value: f64,
    holds: bool,
    message: String,
}

fn baseline_check(metric: &str, value: f64, deviation: Option<Deviation>) -> Check {
    let message = deviation
        .map(|d| {
            format!(
                "{metric} = {value:.1}, обычно ≈ {:.1} ({:+.1}σ)",
                d.mean, d.sigmas
            )
        })
        .unwrap_or_default();
    Check {
        rule_id: format!("{BASELINE_RULE_PREFIX}{metric}"),
        rule_name: format!("Аномалия {metric}"),
        severity: Severity::Warning,
        for_secs: BASELINE_FOR_SECS,
        value,
        holds: deviation.is_some(),
        message,
    }
}

fn metric_value(server: &ServerState, metric: &str) -> Option<f64> {
    if metric == METRIC_OFFLINE {
        return Some(match &server.connection {
            ConnectionStatus::Offline { .. } => 1.0,
            _ => 0.0,
        });
    }
    server.latest_value(metric)
}

#[cfg(test)]
mod tests {
    use super::*;
    use asiba_core::{AlertRule, AuthMethod, Condition, Point, ServerSpec, SudoMode};
    use chrono::Duration;

    fn spec(id: ServerId) -> ServerSpec {
        ServerSpec {
            id,
            host: "h".into(),
            port: 22,
            user: "u".into(),
            auth: AuthMethod::Auto,
            jump: None,
            sudo: SudoMode::None,
            description: Default::default(),
        }
    }

    fn state_with(server: &str, metric: &str, value: f64, at: DateTime<Utc>) -> AppState {
        let mut state = AppState::default();
        let id = ServerId::parse(server).expect("id");
        let mut server_state = ServerState::new(spec(id.clone()));
        server_state.connection = ConnectionStatus::Online { since: at };
        server_state
            .series
            .entry(metric.to_owned())
            .or_default()
            .push(Point { at, value });
        state.servers.insert(id, server_state);
        state
    }

    fn cpu_rule(for_secs: u64) -> AlertRule {
        AlertRule {
            id: "cpu".to_owned(),
            name: "CPU".to_owned(),
            metric: cpu::KEY_TOTAL.to_owned(),
            condition: Condition::Above,
            threshold: 90.0,
            for_secs,
            severity: Severity::Warning,
            builtin: true,
        }
    }

    #[test]
    fn rule_without_duration_raises_immediately_and_resolves() {
        let now = Utc::now();
        let mut state = state_with("neo", cpu::KEY_TOTAL, 95.0, now);
        let mut evaluator = Evaluator::new(vec![cpu_rule(0)]);
        let raised = evaluator.evaluate(&mut state, now);
        assert_eq!(raised.0.len(), 1);
        assert_eq!(state.active_alerts().count(), 1);
        let later = now + Duration::seconds(10);
        state = state_with("neo", cpu::KEY_TOTAL, 10.0, later);
        state.alerts = raised.0.clone();
        evaluator.evaluate(&mut state, later);
        assert_eq!(state.active_alerts().count(), 0);
        assert!(state.alerts[0].resolved_at.is_some());
    }

    #[test]
    fn rule_with_duration_waits_before_raising() {
        let now = Utc::now();
        let mut state = state_with("neo", cpu::KEY_TOTAL, 95.0, now);
        let mut evaluator = Evaluator::new(vec![cpu_rule(60)]);
        assert!(evaluator.evaluate(&mut state, now).0.is_empty());
        let later = now + Duration::seconds(61);
        let raised = evaluator.evaluate(&mut state, later);
        assert_eq!(raised.0.len(), 1);
        assert_eq!(raised.0[0].started_at, now);
        assert!(evaluator.evaluate(&mut state, later).0.is_empty());
    }

    #[test]
    fn offline_metric_is_derived_from_connection() {
        let now = Utc::now();
        let mut state = state_with("neo", cpu::KEY_TOTAL, 1.0, now);
        if let Some(server) = state.servers.values_mut().next() {
            server.connection = ConnectionStatus::Offline {
                reason: "x".to_owned(),
                retry_at: now,
            };
        }
        let mut evaluator = Evaluator::new(crate::builtin_rules());
        let later = now + Duration::seconds(61);
        evaluator.evaluate(&mut state, now);
        let raised = evaluator.evaluate(&mut state, later);
        assert!(raised.0.iter().any(|a| a.rule_id == "offline"));
    }
}
