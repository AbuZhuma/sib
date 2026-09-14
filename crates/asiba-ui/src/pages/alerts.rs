use egui::Ui;

use crate::components::page_title;
use crate::text;

pub fn show(ui: &mut Ui) {
    page_title(ui, text::ALERTS_TITLE);
    ui.label(text::ALERTS_PLACEHOLDER);
}
