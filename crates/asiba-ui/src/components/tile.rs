use egui::{Color32, Frame, Margin, RichText, Stroke, Ui, Vec2};

use crate::theme::{GAP, Palette};

const TILE_SIZE: Vec2 = Vec2::new(150.0, 64.0);

pub struct TileSpec {
    pub label: &'static str,
    pub value: String,
    pub color: Option<Color32>,
}

pub fn tile_grid(ui: &mut Ui, tiles: &[TileSpec]) {
    let step = TILE_SIZE.x + 2.0 * GAP + GAP;
    let per_row = ((ui.available_width() + GAP) / step).floor().max(1.0) as usize;
    for row in tiles.chunks(per_row) {
        ui.horizontal(|ui| {
            for spec in row {
                tile(ui, spec.label, &spec.value, spec.color);
            }
        });
    }
}

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
