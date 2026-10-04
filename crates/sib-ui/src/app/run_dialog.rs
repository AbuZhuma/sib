use std::collections::BTreeMap;

use egui::{Grid, Modal, RichText, TextEdit, Ui};
use sib_core::{Environment, Pipeline, ServerId};
use sib_engine::Command;

use super::SibApp;
use crate::text;
use crate::theme::{FIELD_WIDTH, GAP, Palette, ROW_HEIGHT};

const DIALOG_WIDTH: f32 = 520.0;

pub struct RunDialog {
    pub server: ServerId,
    pub pipeline: Pipeline,
    pub values: BTreeMap<String, String>,
    pub needs_name: bool,
    pub typed: String,
    pub is_online: bool,
}

impl RunDialog {
    pub fn new(
        server: ServerId,
        pipeline: Pipeline,
        bound: &BTreeMap<String, String>,
        environment: Environment,
        is_online: bool,
    ) -> Self {
        let values = pipeline
            .variables
            .iter()
            .map(|v| {
                let value = bound
                    .get(&v.name)
                    .cloned()
                    .unwrap_or_else(|| v.default.clone());
                (v.name.clone(), value)
            })
            .collect();
        Self {
            server,
            pipeline,
            values,
            needs_name: environment == Environment::Production,
            typed: String::new(),
            is_online,
        }
    }

    fn is_confirmed(&self) -> bool {
        !self.needs_name || self.typed.trim() == self.server.as_str()
    }

    fn body(&mut self, ui: &mut Ui) {
        let p = Palette::current(ui.ctx());
        ui.heading(text::RUN_DIALOG_TITLE);
        ui.label(
            RichText::new(format!(
                "{} · {} · {} {}",
                self.pipeline.name,
                self.server,
                self.pipeline.steps.len(),
                text::RUN_DIALOG_STEPS
            ))
            .color(p.text_secondary),
        );
        if !self.is_online {
            ui.label(RichText::new(text::ERR_SERVER_OFFLINE).color(p.warning));
        }
        ui.add_space(GAP);
        if self.pipeline.variables.is_empty() {
            ui.label(RichText::new(text::RUN_DIALOG_NO_VARIABLES).color(p.text_muted));
        }
        Grid::new("run-dialog-values")
            .num_columns(2)
            .spacing([GAP, GAP])
            .show(ui, |ui| {
                for variable in &self.pipeline.variables {
                    ui.label(RichText::new(&variable.name).monospace());
                    let value = self.values.entry(variable.name.clone()).or_default();
                    ui.add_sized(
                        [FIELD_WIDTH, ROW_HEIGHT],
                        TextEdit::singleline(value)
                            .password(variable.secret)
                            .hint_text(&variable.description),
                    );
                    ui.end_row();
                }
            });
        if self.needs_name {
            ui.add_space(GAP);
            ui.label(RichText::new(text::CONFIRM_PROMPT).color(p.warning));
            ui.text_edit_singleline(&mut self.typed);
        }
    }
}

impl SibApp {
    pub(super) fn run_modal(&mut self, ctx: &egui::Context) {
        let Some(dialog) = &mut self.run_dialog else {
            return;
        };
        let mut close = false;
        let mut confirmed = false;
        Modal::new(egui::Id::new("run-pipeline")).show(ctx, |ui| {
            ui.set_width(DIALOG_WIDTH);
            dialog.body(ui);
            ui.add_space(GAP);
            ui.horizontal(|ui| {
                if ui
                    .add_enabled(dialog.is_confirmed(), egui::Button::new(text::BTN_RUN))
                    .clicked()
                {
                    confirmed = true;
                }
                if ui.button(text::BTN_CANCEL).clicked() {
                    close = true;
                }
            });
        });
        if confirmed && let Some(dialog) = self.run_dialog.take() {
            self.engine.send(Command::RunPipeline {
                server: dialog.server,
                pipeline: dialog.pipeline,
                values: dialog.values,
            });
            return;
        }
        if close {
            self.run_dialog = None;
        }
    }
}
