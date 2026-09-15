use std::time::{Duration, Instant};

use asiba_core::{AppState, ConnectionStatus, Severity};
use asiba_modules::anomalies::{self, AnomaliesSnapshot};
use egui::{RichText, Ui};

use crate::components::{badge, status_dot};
use crate::format;
use crate::text;
use crate::theme::Palette;

const NOTICE_TTL: Duration = Duration::from_secs(12);

#[derive(Debug)]
pub struct Notice {
    pub message: String,
    pub at: Instant,
}

impl Notice {
    pub fn new(message: String) -> Self {
        Self {
            message,
            at: Instant::now(),
        }
    }

    pub fn is_expired(&self) -> bool {
        self.at.elapsed() > NOTICE_TTL
    }
}

pub struct StatusContext<'a> {
    pub state: &'a AppState,
    pub notices: &'a [Notice],
    pub paused: &'a mut bool,
}

pub fn statusbar(ui: &mut Ui, ctx: StatusContext<'_>) {
    let p = Palette::current(ui.ctx());
    let state = ctx.state;
    let online = state.online_count();
    let total = state.servers.len();
    let connecting = state
        .servers
        .values()
        .filter(|s| matches!(s.connection, ConnectionStatus::Connecting))
        .count();
    ui.horizontal_centered(|ui| {
        let all_online = online == total && total > 0;
        status_dot(ui, if all_online { p.ok } else { p.warning });
        ui.monospace(format!("{online}/{total} {}", text::STATUS_ONLINE));
        if connecting > 0 {
            ui.monospace(
                RichText::new(format!("{connecting} {}", text::STATUS_CONNECTING)).color(p.warning),
            );
        }
        alert_counters(ui, state, &p);
        if is_under_attack(state) {
            badge(ui, text::STATUS_ATTACK, p.critical);
        }
        let queued = state
            .audits
            .iter()
            .filter(|a| a.status == asiba_core::AuditStatus::Queued)
            .count();
        if state.audits.iter().any(|a| a.is_running()) {
            let label = if queued > 0 {
                format!(
                    "{} · {} {queued}",
                    text::STATUS_AI_BUSY,
                    text::STATUS_AI_QUEUED
                )
            } else {
                text::STATUS_AI_BUSY.to_owned()
            };
            badge(ui, &label, p.accent);
        }
        for notice in ctx.notices.iter().rev().take(2) {
            ui.separator();
            ui.label(RichText::new(&notice.message).color(p.warning));
        }
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            pause_button(ui, ctx.paused, &p);
            let last = last_update(state);
            let label = if *ctx.paused {
                text::STATUS_PAUSED.to_owned()
            } else {
                format!("{} {last}", text::STATUS_LIVE)
            };
            ui.monospace(RichText::new(label).color(p.text_muted));
        });
    });
}

fn alert_counters(ui: &mut Ui, state: &AppState, p: &Palette) {
    let count = |severity: Severity| {
        state
            .active_alerts()
            .filter(|a| a.severity == severity)
            .count()
    };
    for (severity, color) in [
        (Severity::Critical, p.critical),
        (Severity::Warning, p.warning),
    ] {
        let n = count(severity);
        if n > 0 {
            ui.separator();
            status_dot(ui, color);
            ui.monospace(RichText::new(n.to_string()).color(color));
        }
    }
}

fn is_under_attack(state: &AppState) -> bool {
    state
        .servers
        .values()
        .filter_map(|s| s.data::<AnomaliesSnapshot>(anomalies::ID))
        .any(|a| a.is_under_attack())
}

fn last_update(state: &AppState) -> String {
    state
        .servers
        .values()
        .flat_map(|s| s.modules.values())
        .filter_map(|m| m.last_collected)
        .max()
        .map(format::clock)
        .unwrap_or_default()
}

fn pause_button(ui: &mut Ui, paused: &mut bool, p: &Palette) {
    let label = if *paused {
        RichText::new(text::BTN_RESUME).color(p.warning)
    } else {
        RichText::new(text::BTN_PAUSE).color(p.text_secondary)
    };
    if ui.small_button(label).clicked() {
        *paused = !*paused;
    }
}
