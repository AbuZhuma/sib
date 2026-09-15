use egui::{Frame, Margin, RichText, Stroke, Ui};

use crate::theme::{GAP, GAP_SMALL, Palette};

pub fn panel<R>(ui: &mut Ui, title: &str, add_contents: impl FnOnce(&mut Ui) -> R) -> R {
    let p = Palette::current(ui.ctx());
    Frame::new()
        .fill(p.bg_panel)
        .stroke(Stroke::new(1.0, p.border))
        .inner_margin(Margin::same(GAP as i8))
        .show(ui, |ui| {
            ui.set_min_width(ui.available_width());
            ui.label(
                RichText::new(title.to_uppercase())
                    .small()
                    .color(p.text_secondary),
            );
            ui.add_space(GAP_SMALL);
            add_contents(ui)
        })
        .inner
}

pub fn panel_plain<R>(ui: &mut Ui, add_contents: impl FnOnce(&mut Ui) -> R) -> R {
    let p = Palette::current(ui.ctx());
    Frame::new()
        .fill(p.bg_panel)
        .stroke(Stroke::new(1.0, p.border))
        .inner_margin(Margin::same(GAP as i8))
        .show(ui, |ui| {
            ui.set_min_width(ui.available_width());
            add_contents(ui)
        })
        .inner
}

pub fn panel_with_controls<R>(
    ui: &mut Ui,
    title: &str,
    controls: impl FnOnce(&mut Ui),
    add_contents: impl FnOnce(&mut Ui) -> R,
) -> R {
    let p = Palette::current(ui.ctx());
    Frame::new()
        .fill(p.bg_panel)
        .stroke(Stroke::new(1.0, p.border))
        .inner_margin(Margin::same(GAP as i8))
        .show(ui, |ui| {
            ui.set_min_width(ui.available_width());
            ui.horizontal(|ui| {
                ui.label(
                    RichText::new(title.to_uppercase())
                        .small()
                        .color(p.text_secondary),
                );
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), controls);
            });
            ui.add_space(GAP_SMALL);
            add_contents(ui)
        })
        .inner
}

pub fn page_title(ui: &mut Ui, title: &str) {
    ui.add_space(GAP_SMALL);
    ui.heading(title);
    ui.add_space(GAP);
}
