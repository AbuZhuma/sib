use egui::{Color32, RichText, Sense, Stroke, Ui, Vec2};

use crate::theme::Palette;

const HEIGHT: f32 = 8.0;
const WARNING_AT: f64 = 80.0;
const CRITICAL_AT: f64 = 92.0;

pub fn meter(ui: &mut Ui, label: &str, percent: f64, detail: &str) {
    let p = Palette::current(ui.ctx());
    ui.horizontal(|ui| {
        ui.label(RichText::new(label).color(p.text_secondary));
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            ui.monospace(detail);
            ui.monospace(format!("{percent:>3.0}%"));
        });
    });
    let width = ui.available_width();
    let (rect, _) = ui.allocate_exact_size(Vec2::new(width, HEIGHT), Sense::hover());
    let painter = ui.painter();
    painter.rect_filled(rect, 0.0, p.bg_window);
    let mut filled = rect;
    filled.set_width(width * (percent.clamp(0.0, 100.0) / 100.0) as f32);
    painter.rect_filled(filled, 0.0, level_color(&p, percent));
    painter.rect_stroke(
        rect,
        0.0,
        Stroke::new(1.0, p.border),
        egui::StrokeKind::Inside,
    );
}

pub fn level_color(p: &Palette, percent: f64) -> Color32 {
    if percent >= CRITICAL_AT {
        return p.critical;
    }
    if percent >= WARNING_AT {
        return p.warning;
    }
    p.accent
}
