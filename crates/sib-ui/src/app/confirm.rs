use egui::{ComboBox, Modal, RichText, Ui};
use sib_core::{ActionRequest, ActionSpec, Danger, Environment, ServerId};
use sib_engine::Command;
use sib_modules::security;

use super::SibApp;
use crate::components::chip;
use crate::text;
use crate::theme::Palette;

const DIALOG_WIDTH: f32 = 420.0;
const BAN_DURATIONS: [(&str, &str); 5] = [
    ("1h", "1 час"),
    ("12h", "12 часов"),
    ("24h", "24 часа"),
    ("7d", "7 дней"),
    (security::PERMANENT, text::CONFIRM_FOREVER),
];
const DEFAULT_BAN_DURATION: &str = "24h";

pub struct ConfirmDialog {
    pub server: ServerId,
    pub spec: ActionSpec,
    pub request: ActionRequest,
    pub needs_name: bool,
    pub typed: String,
}

impl ConfirmDialog {
    pub fn new(
        server: ServerId,
        spec: ActionSpec,
        request: ActionRequest,
        environment: Environment,
    ) -> Self {
        let is_ban = spec.module == security::ID && request.kind == security::ACTION_BAN;
        let request = if is_ban && request.argument.is_none() {
            request.with_argument(DEFAULT_BAN_DURATION)
        } else {
            request
        };
        Self {
            server,
            spec,
            needs_name: spec.danger == Danger::High || environment == Environment::Production,
            request,
            typed: String::new(),
        }
    }

    fn is_confirmed(&self) -> bool {
        !self.needs_name || self.typed.trim() == self.server.as_str()
    }

    fn has_duration(&self) -> bool {
        self.spec.module == security::ID && self.request.kind == security::ACTION_BAN
    }

    fn body(&mut self, ui: &mut Ui) {
        let p = Palette::current(ui.ctx());
        ui.heading(self.spec.title);
        egui::Grid::new("confirm-grid")
            .num_columns(2)
            .spacing([12.0, 4.0])
            .show(ui, |ui| {
                ui.label(RichText::new(text::CONFIRM_SERVER).color(p.text_secondary));
                ui.monospace(self.server.as_str());
                ui.end_row();
                ui.label(RichText::new(text::CONFIRM_TARGET).color(p.text_secondary));
                ui.monospace(&self.request.target);
                ui.end_row();
                if self.has_duration() {
                    ui.label(RichText::new(text::CONFIRM_DURATION).color(p.text_secondary));
                    duration_picker(ui, &mut self.request);
                    ui.end_row();
                }
            });
        if self.needs_name {
            ui.label(RichText::new(text::CONFIRM_PROMPT).color(p.warning));
            ui.text_edit_singleline(&mut self.typed);
        }
    }
}

fn duration_picker(ui: &mut Ui, request: &mut ActionRequest) {
    let current = request.argument.clone().unwrap_or_default();
    let label = BAN_DURATIONS
        .iter()
        .find(|(value, _)| *value == current)
        .map(|(_, label)| *label)
        .unwrap_or(text::CONFIRM_FOREVER);
    ComboBox::from_id_salt("ban-duration")
        .selected_text(label)
        .show_ui(ui, |ui| {
            for (value, label) in BAN_DURATIONS {
                if chip(ui, current == value, label).clicked() {
                    request.argument = Some(value.to_owned());
                }
            }
        });
}

impl SibApp {
    pub(super) fn confirm_modal(&mut self, ctx: &egui::Context) {
        let Some(dialog) = &mut self.confirm_dialog else {
            return;
        };
        let mut close = false;
        let mut confirmed = false;
        Modal::new(egui::Id::new("confirm-action")).show(ctx, |ui| {
            ui.set_width(DIALOG_WIDTH);
            dialog.body(ui);
            ui.horizontal(|ui| {
                let enabled = dialog.is_confirmed();
                if ui
                    .add_enabled(enabled, egui::Button::new(text::BTN_CONFIRM))
                    .clicked()
                {
                    confirmed = true;
                }
                if ui.button(text::BTN_CANCEL).clicked() {
                    close = true;
                }
            });
        });
        if confirmed && let Some(dialog) = self.confirm_dialog.take() {
            self.engine.send(Command::Perform {
                server: dialog.server,
                module: dialog.spec.module,
                request: dialog.request,
            });
            return;
        }
        if close {
            self.confirm_dialog = None;
        }
    }
}
