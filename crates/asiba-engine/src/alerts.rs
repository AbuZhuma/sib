use std::time::Duration;

use asiba_alerts::Evaluator;
use asiba_core::{AlertRule, SharedState};
use chrono::{DateTime, Utc};
use tokio::sync::watch;
use tokio::time::{MissedTickBehavior, interval};

use crate::engine::RepaintNotifier;

const EVALUATE_INTERVAL: Duration = Duration::from_secs(5);

#[derive(Debug, Clone, PartialEq)]
pub struct AlertSettings {
    pub rules: Vec<AlertRule>,
    pub desktop_notifications: bool,
}

impl AlertSettings {
    pub fn from_custom(custom: &[AlertRule], desktop_notifications: bool) -> Self {
        Self {
            rules: asiba_alerts::merge_rules(custom),
            desktop_notifications,
        }
    }
}

pub struct AlertLoop {
    pub state: SharedState,
    pub settings: watch::Receiver<AlertSettings>,
    pub notify: RepaintNotifier,
}

pub fn spawn(context: AlertLoop) {
    tokio::spawn(run(context));
}

async fn run(mut context: AlertLoop) {
    let mut ticker = interval(EVALUATE_INTERVAL);
    ticker.set_missed_tick_behavior(MissedTickBehavior::Delay);
    let mut evaluator = Evaluator::new(context.settings.borrow().rules.clone());
    loop {
        ticker.tick().await;
        if context.settings.has_changed().unwrap_or(false) {
            let settings = context.settings.borrow_and_update().clone();
            evaluator.set_rules(settings.rules);
        }
        let raised = match context.state.write() {
            Ok(mut state) => evaluator.evaluate(&mut state, Utc::now()).0,
            Err(_) => continue,
        };
        if raised.is_empty() {
            continue;
        }
        let notify_desktop = context.settings.borrow().desktop_notifications;
        for alert in &raised {
            tracing::info!(server = %alert.server, rule = %alert.rule_id, "алерт: {}", alert.message);
            if notify_desktop && alert.severity > asiba_core::Severity::Info {
                asiba_alerts::send_desktop(alert);
            }
        }
        (context.notify)();
    }
}

pub fn acknowledge(state: &SharedState, id: u64) {
    if let Ok(mut state) = state.write()
        && let Some(alert) = state.alert_mut(id)
    {
        alert.acknowledged = true;
    }
}

pub fn mute(state: &SharedState, id: u64, until: DateTime<Utc>) {
    if let Ok(mut state) = state.write()
        && let Some(alert) = state.alert_mut(id)
    {
        alert.muted_until = Some(until);
        alert.acknowledged = true;
    }
}
