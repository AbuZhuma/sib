use asiba_core::ServerId;
use asiba_modules::users::{COMMAND_PREFIX, HEADING_PREFIX};
use egui::{Label, RichText, Ui};

use super::Action;
use crate::components::scroll;
use crate::text;
use crate::theme::{GAP, Palette};

const MAX_HEIGHT: f32 = 320.0;

#[derive(Debug, Clone)]
pub struct Inspector {
    pub token: u64,
    pub server: ServerId,
    pub title: String,
    pub result: Option<Result<String, String>>,
}

impl Inspector {
    pub fn accept(&mut self, token: u64, result: Result<(String, String), String>) {
        if token != self.token {
            return;
        }
        match result {
            Ok((title, text)) => {
                self.title = title;
                self.result = Some(Ok(text));
            }
            Err(error) => self.result = Some(Err(error)),
        }
    }
}

pub fn show(ui: &mut Ui, inspector: &Inspector) -> Option<Action> {
    let p = Palette::current(ui.ctx());
    let mut action = None;
    ui.horizontal(|ui| {
        ui.label(RichText::new(&inspector.title).strong());
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            if ui.button(text::BTN_CLOSE).clicked() {
                action = Some(Action::CloseInspector);
            }
        });
    });
    ui.add_space(GAP);
    match &inspector.result {
        None => {
            ui.label(RichText::new(text::INSPECTOR_LOADING).color(p.text_muted));
        }
        Some(Err(error)) => {
            ui.label(RichText::new(error).color(p.critical));
        }
        Some(Ok(body)) => {
            scroll::vertical().max_height(MAX_HEIGHT).show(ui, |ui| {
                ui.set_min_width(ui.available_width());
                ui.spacing_mut().item_spacing.y = 0.0;
                for line in body.lines() {
                    ui.add(Label::new(styled_line(line, &p)).wrap());
                }
            });
        }
    }
    action
}

fn styled_line(line: &str, p: &Palette) -> RichText {
    if let Some(command) = line.strip_prefix(COMMAND_PREFIX) {
        return RichText::new(format!("{COMMAND_PREFIX}{command}"))
            .monospace()
            .color(p.accent);
    }
    if let Some(heading) = line.strip_prefix(HEADING_PREFIX) {
        return RichText::new(heading.to_uppercase())
            .small()
            .color(p.text_secondary);
    }
    RichText::new(line).monospace().color(p.text)
}
