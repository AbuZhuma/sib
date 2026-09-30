use egui::{Button, Response, RichText, Stroke, Ui, Vec2};

use crate::theme::{Palette, ROW_HEIGHT};

pub fn nav_item(ui: &mut Ui, label: &str, selected: bool) -> Response {
    let p = Palette::current(ui.ctx());
    let (fill, stroke, color) = if selected {
        (p.accent_bg, Stroke::new(1.0, p.accent), p.text)
    } else {
        (p.bg_window, Stroke::NONE, p.text_secondary)
    };
    let button = Button::new(RichText::new(label).color(color))
        .fill(fill)
        .stroke(stroke)
        .min_size(Vec2::new(ui.available_width(), ROW_HEIGHT));
    ui.add(button)
}
