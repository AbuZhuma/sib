use egui::{RichText, TextEdit, Ui};

use crate::theme::{FIELD_WIDTH, Palette};

pub fn section_label(ui: &mut Ui, label: &str) {
    let p = Palette::current(ui.ctx());
    ui.add_space(4.0);
    ui.label(
        RichText::new(label.to_uppercase())
            .small()
            .color(p.text_secondary),
    );
}

pub fn field(ui: &mut Ui, label: &str, value: &mut String, hint: &str) {
    labelled(ui, label, |ui| {
        ui.add(
            TextEdit::singleline(value)
                .desired_width(FIELD_WIDTH)
                .hint_text(hint),
        );
    });
}

pub fn password_field(ui: &mut Ui, label: &str, value: &mut String, hint: &str) {
    labelled(ui, label, |ui| {
        ui.add(
            TextEdit::singleline(value)
                .desired_width(FIELD_WIDTH)
                .hint_text(hint)
                .password(true),
        );
    });
}

pub fn labelled(ui: &mut Ui, label: &str, add_contents: impl FnOnce(&mut Ui)) {
    let p = Palette::current(ui.ctx());
    ui.label(RichText::new(label).color(p.text_secondary));
    add_contents(ui);
    ui.end_row();
}
