use asiba_core::ServerId;
use egui::{RichText, ScrollArea, TextEdit, Ui};

use super::Action;
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
            ScrollArea::vertical()
                .max_height(MAX_HEIGHT)
                .show(ui, |ui| {
                    let mut text = body.as_str();
                    ui.add(
                        TextEdit::multiline(&mut text)
                            .font(egui::TextStyle::Monospace)
                            .desired_width(f32::INFINITY),
                    );
                });
        }
    }
    action
}
