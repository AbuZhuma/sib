use std::time::{Duration, Instant};

use egui::{RichText, Ui};
use sib_core::{AppState, ConnectionStatus, Severity};
use sib_modules::anomalies::{self, AnomaliesSnapshot};

use crate::components::{badge, status_dot};
use crate::format;
use crate::text;
use crate::theme::Palette;

const NOTICE_TTL: Duration = Duration::from_secs(12);
const MAX_NOTICES: usize = 2;
const RIGHT_BLOCK_WIDTH: f32 = 220.0;

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
        ai_badge(ui, state, &p);
        notices(ui, ctx.notices, &p);
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            pause_button(ui, ctx.paused, &p);
            live_label(ui, state, *ctx.paused, &p);
        });
    });
}

fn notices(ui: &mut Ui, notices: &[Notice], p: &Palette) {
    let shown: Vec<&Notice> = notices.iter().rev().take(MAX_NOTICES).collect();
    if shown.is_empty() {
        return;
    }
    let available = (ui.available_width() - RIGHT_BLOCK_WIDTH).max(0.0);
    let width = available / shown.len() as f32;
    for notice in shown {
        ui.separator();
        ui.add_sized(
            [width, ui.available_height()],
            egui::Label::new(RichText::new(&notice.message).color(p.warning)).truncate(),
        )
        .on_hover_text(&notice.message);
    }
}

fn ai_badge(ui: &mut Ui, state: &AppState, p: &Palette) {
    if !state.audits.iter().any(|a| a.is_running()) {
        return;
    }
    let queued = state
        .audits
        .iter()
        .filter(|a| a.status == sib_core::AuditStatus::Queued)
        .count();
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

fn live_label(ui: &mut Ui, state: &AppState, paused: bool, p: &Palette) {
    let label = if paused {
        text::STATUS_PAUSED.to_owned()
    } else {
        format!("{} {}", text::STATUS_LIVE, last_update(state))
    };
    ui.monospace(RichText::new(label).color(p.text_muted));
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
