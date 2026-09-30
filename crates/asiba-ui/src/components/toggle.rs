use egui::{Response, RichText, Sense, Stroke, Ui, Vec2, pos2};

use crate::theme::{GAP_SMALL, Palette};

const WIDTH: f32 = 32.0;
const HEIGHT: f32 = 16.0;
const KNOB_INSET: f32 = 2.0;

pub fn toggle(ui: &mut Ui, value: &mut bool, label: &str) -> Response {
    let p = Palette::current(ui.ctx());
    let response = ui
        .horizontal(|ui| {
            let switch = switch(ui, *value, &p);
            ui.add_space(GAP_SMALL);
            let text = ui.label(RichText::new(label).color(p.text));
            switch | text.interact(Sense::click())
        })
        .inner;
    if response.clicked() {
        *value = !*value;
    }
    response
}

fn switch(ui: &mut Ui, is_on: bool, p: &Palette) -> Response {
    let (rect, response) = ui.allocate_exact_size(Vec2::new(WIDTH, HEIGHT), Sense::click());
    let travel = ui.ctx().animate_bool(response.id, is_on);
    let track = if is_on { p.accent } else { p.bg_window };
    let painter = ui.painter();
    let radius = HEIGHT / 2.0;
    painter.rect_filled(rect, radius, track);
    painter.rect_stroke(
        rect,
        radius,
        Stroke::new(1.0, p.border),
        egui::StrokeKind::Inside,
    );
    let knob = radius - KNOB_INSET;
    let left = rect.left() + radius;
    let center = pos2(left + (WIDTH - HEIGHT) * travel, rect.center().y);
    painter.circle_filled(center, knob, knob_color(p, is_on));
    response
}

fn knob_color(p: &Palette, is_on: bool) -> egui::Color32 {
    if is_on { p.bg_window } else { p.text_muted }
}
