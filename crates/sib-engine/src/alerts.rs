use std::time::Duration;

use chrono::{DateTime, Utc};
use sib_alerts::Evaluator;
use sib_config::AiConfig;
use sib_core::{AlertRule, AuditScope, AuditTarget, SharedState};
use tokio::sync::{mpsc, watch};
use tokio::time::{MissedTickBehavior, interval};

use crate::command::{Command, EngineEvent};
use crate::engine::RepaintNotifier;
use crate::incidents;

const EVALUATE_INTERVAL: Duration = Duration::from_secs(5);

#[derive(Debug, Clone, PartialEq)]
pub struct AlertSettings {
    pub rules: Vec<AlertRule>,
    pub desktop_notifications: bool,
}

impl AlertSettings {
    pub fn from_custom(custom: &[AlertRule], desktop_notifications: bool) -> Self {
        Self {
            rules: sib_alerts::merge_rules(custom),
            desktop_notifications,
        }
    }
}

pub struct AlertLoop {
    pub state: SharedState,
    pub settings: watch::Receiver<AlertSettings>,
    pub ai: watch::Receiver<AiConfig>,
    pub notify: RepaintNotifier,
    pub events: mpsc::UnboundedSender<EngineEvent>,
    pub commands: mpsc::UnboundedSender<Command>,
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
        let now = Utc::now();
        let (raised, opened) = match context.state.write() {
            Ok(mut state) => {
                let raised = evaluator.evaluate(&mut state, now).0;
                let opened = incidents::reconcile(&mut state, now);
                (raised, opened)
            }
            Err(_) => continue,
        };
        if raised.is_empty() && opened.is_empty() {
            continue;
        }
        let notify_desktop = context.settings.borrow().desktop_notifications;
        for alert in &raised {
            tracing::info!(server = %alert.server, rule = %alert.rule_id, "alert: {}", alert.message);
            if notify_desktop && alert.severity > sib_core::Severity::Info {
                sib_alerts::send_desktop(alert);
            }
        }
        incidents::announce(&opened, notify_desktop);
        request_audits(&context, &opened);
        let _ = context.events.send(EngineEvent::IncidentsOpened(opened));
        (context.notify)();
    }
}

fn request_audits(context: &AlertLoop, opened: &[sib_core::Incident]) {
    let config = context.ai.borrow();
    if !config.is_ready() || !config.auto_audit {
        return;
    }
    for incident in opened {
        let _ = context.commands.send(Command::Audit {
            target: AuditTarget::Server(incident.server.clone()),
            scope: AuditScope::Incident {
                incident_id: incident.id,
                kind: incident.kind,
                subject: incident.subject.clone(),
            },
            is_auto: true,
        });
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
