use asiba_core::Severity;
use egui::{Color32, CornerRadius, Frame, Margin, RichText, Stroke, Ui};

use crate::theme::Palette;

pub fn badge(ui: &mut Ui, label: &str, color: Color32) {
    Frame::new()
        .stroke(Stroke::new(1.0, color))
        .corner_radius(CornerRadius::same(2))
        .inner_margin(Margin::symmetric(5, 1))
        .show(ui, |ui| {
            ui.label(RichText::new(label).small().color(color));
        });
}

pub fn severity_color(p: &Palette, severity: Severity) -> Color32 {
    match severity {
        Severity::Info => p.info,
        Severity::Warning => p.warning,
        Severity::Critical => p.critical,
    }
}
