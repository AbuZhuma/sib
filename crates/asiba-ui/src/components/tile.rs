use egui::{Color32, Frame, Margin, RichText, Stroke, Ui, Vec2};

use crate::theme::{GAP, Palette};

const TILE_SIZE: Vec2 = Vec2::new(150.0, 64.0);

pub fn tile(ui: &mut Ui, label: &str, value: &str, color: Option<Color32>) {
    let p = Palette::current(ui.ctx());
    Frame::new()
        .fill(p.bg_panel)
        .stroke(Stroke::new(1.0, p.border))
        .inner_margin(Margin::same(GAP as i8))
        .show(ui, |ui| {
            ui.set_min_size(TILE_SIZE);
            ui.set_max_size(TILE_SIZE);
            ui.vertical(|ui| {
                ui.label(
                    RichText::new(label.to_uppercase())
                        .small()
                        .color(p.text_secondary),
                );
                let size = if value.chars().count() > 8 {
                    15.0
                } else {
                    24.0
                };
                ui.label(
                    RichText::new(value)
                        .size(size)
                        .monospace()
                        .color(color.unwrap_or(p.text)),
                );
            });
        });
}
