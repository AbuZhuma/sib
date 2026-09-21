use egui::{Color32, Frame, Margin, RichText, Stroke, Ui, Vec2};

use crate::theme::{GAP, Palette};

const TILE_HEIGHT: f32 = 64.0;
const MIN_TILE_WIDTH: f32 = 110.0;
const MAX_TILE_WIDTH: f32 = 220.0;
const VALUE_SIZE: f32 = 24.0;
const SMALL_VALUE_SIZE: f32 = 15.0;

pub struct TileSpec {
    pub label: &'static str,
    pub value: String,
    pub color: Option<Color32>,
}

pub fn tile_grid(ui: &mut Ui, tiles: &[TileSpec]) {
    let spacing = ui.spacing().item_spacing.x;
    let available = ui.available_width();
    let min_outer = MIN_TILE_WIDTH + 2.0 * GAP;
    let per_row = (((available + spacing) / (min_outer + spacing)).floor() as usize)
        .clamp(1, tiles.len().max(1));
    let fitted = (available - spacing * (per_row as f32 - 1.0)) / per_row as f32 - 2.0 * GAP;
    let width = fitted.clamp(MIN_TILE_WIDTH, MAX_TILE_WIDTH);
    for row in tiles.chunks(per_row) {
        ui.horizontal(|ui| {
            for spec in row {
                tile(ui, spec, width);
            }
        });
    }
}

fn tile(ui: &mut Ui, spec: &TileSpec, width: f32) {
    let p = Palette::current(ui.ctx());
    let size = Vec2::new(width, TILE_HEIGHT);
    Frame::new()
        .fill(p.bg_panel)
        .stroke(Stroke::new(1.0, p.border))
        .inner_margin(Margin::same(GAP as i8))
        .show(ui, |ui| {
            ui.set_min_size(size);
            ui.set_max_size(size);
            ui.vertical(|ui| {
                ui.label(
                    RichText::new(spec.label.to_uppercase())
                        .small()
                        .color(p.text_secondary),
                );
                let font = if spec.value.chars().count() > 8 {
                    SMALL_VALUE_SIZE
                } else {
                    VALUE_SIZE
                };
                ui.add(
                    egui::Label::new(
                        RichText::new(&spec.value)
                            .size(font)
                            .monospace()
                            .color(spec.color.unwrap_or(p.text)),
                    )
                    .wrap(),
                );
            });
        });
}
