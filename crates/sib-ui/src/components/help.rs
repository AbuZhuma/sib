use egui::{Button, Response, Stroke, Ui};

use crate::theme::{GAP_SMALL, Palette};

const MARK: &str = "?";
const CHAR_WIDTH: f32 = 7.0;
const MIN_WIDTH: f32 = 180.0;
const MAX_WIDTH: f32 = 520.0;
const PADDING: f32 = 16.0;

pub fn help(ui: &mut Ui, description: &str) -> Response {
    let p = Palette::current(ui.ctx());
    let button = Button::new(MARK)
        .small()
        .fill(p.bg_raised)
        .stroke(Stroke::new(1.0, p.border));
    let width = tooltip_width(description);
    ui.add(button).on_hover_ui(|ui| {
        ui.set_max_width(width);
        ui.label(description);
    })
}

pub fn help_after(ui: &mut Ui, label: &str, description: &str) {
    let p = Palette::current(ui.ctx());
    ui.horizontal(|ui| {
        ui.label(egui::RichText::new(label).color(p.text_secondary));
        ui.add_space(GAP_SMALL);
        help(ui, description);
    });
}

fn tooltip_width(description: &str) -> f32 {
    let single_line = description.chars().count() as f32 * CHAR_WIDTH + PADDING;
    single_line.clamp(MIN_WIDTH, MAX_WIDTH)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tooltip_width_grows_with_text_and_stays_within_bounds() {
        assert_eq!(tooltip_width("short"), MIN_WIDTH);
        assert_eq!(tooltip_width(&"a".repeat(40)), 40.0 * CHAR_WIDTH + PADDING);
        assert_eq!(tooltip_width(&"a".repeat(1000)), MAX_WIDTH);
    }
}
