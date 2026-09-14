use std::time::{Duration, Instant};

use asiba_core::{AppState, ConnectionStatus};
use egui::{RichText, Ui};

use crate::components::status_dot;
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

pub fn statusbar(ui: &mut Ui, state: &AppState, notices: &[Notice]) {
    let p = Palette::current(ui.ctx());
    let online = state.online_count();
    let total = state.servers.len();
    let connecting = state
        .servers
        .values()
        .filter(|s| matches!(s.connection, ConnectionStatus::Connecting))
        .count();
    ui.horizontal_centered(|ui| {
        status_dot(
            ui,
            if online == total && total > 0 {
                p.ok
            } else {
                p.warning
            },
        );
        ui.monospace(format!("{online}/{total} {}", text::STATUS_ONLINE));
        if connecting > 0 {
            ui.monospace(
                RichText::new(format!("{connecting} {}", text::STATUS_CONNECTING)).color(p.warning),
            );
        }
        for notice in notices.iter().rev().take(2) {
            ui.separator();
            ui.label(RichText::new(&notice.message).color(p.warning));
        }
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            ui.monospace(RichText::new(text::STATUS_LIVE).color(p.text_muted));
        });
    });
}
