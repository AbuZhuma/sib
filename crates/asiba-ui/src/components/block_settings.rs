use egui::Ui;

use crate::text;

const GEAR: &str = "⚙";

pub fn gear(ui: &mut Ui, key: &str) -> bool {
    ui.push_id(key, |ui| {
        ui.small_button(GEAR)
            .on_hover_text(text::BLOCK_SETTINGS)
            .clicked()
    })
    .inner
}
