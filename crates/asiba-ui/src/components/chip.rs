use egui::{Button, Color32, Response, Stroke, Ui, WidgetText};

use crate::theme::Palette;

pub fn chip(ui: &mut Ui, selected: bool, label: impl Into<WidgetText>) -> Response {
    let p = Palette::current(ui.ctx());
    let mut button = Button::selectable(selected, label).frame_when_inactive(true);
    if !selected {
        button = button
            .fill(Color32::TRANSPARENT)
            .stroke(Stroke::new(1.0, Color32::TRANSPARENT));
    } else {
        button = button.stroke(Stroke::new(1.0, p.accent));
    }
    ui.add(button)
}

pub fn chip_value<T: PartialEq>(
    ui: &mut Ui,
    current: &mut T,
    value: T,
    label: impl Into<WidgetText>,
) -> Response {
    let response = chip(ui, *current == value, label);
    if response.clicked() {
        *current = value;
    }
    response
}
