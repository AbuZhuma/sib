use egui::{Color32, RichText, Ui, Vec2};
use sib_core::ConnectionStatus;

use crate::text;
use crate::theme::Palette;

const DOT_SIZE: f32 = 8.0;

pub fn status_color(p: &Palette, status: &ConnectionStatus) -> Color32 {
    match status {
        ConnectionStatus::Online { .. } => p.ok,
        ConnectionStatus::Connecting => p.warning,
        ConnectionStatus::Offline { .. } => p.critical,
        ConnectionStatus::UntrustedHostKey { .. } => p.warning,
    }
}

pub fn status_text(status: &ConnectionStatus) -> &'static str {
    match status {
        ConnectionStatus::Online { .. } => text::STATUS_ONLINE,
        ConnectionStatus::Connecting => text::STATUS_CONNECTING,
        ConnectionStatus::Offline { .. } => text::STATUS_OFFLINE,
        ConnectionStatus::UntrustedHostKey { .. } => text::STATUS_UNTRUSTED,
    }
}

pub fn status_dot(ui: &mut Ui, color: Color32) {
    let (rect, _) = ui.allocate_exact_size(Vec2::splat(DOT_SIZE), egui::Sense::hover());
    ui.painter().rect_filled(rect, 0.0, color);
}

pub fn status_label(ui: &mut Ui, status: &ConnectionStatus) {
    let p = Palette::current(ui.ctx());
    let color = status_color(&p, status);
    ui.horizontal(|ui| {
        status_dot(ui, color);
        ui.label(RichText::new(status_text(status)).color(color).monospace());
    });
}
