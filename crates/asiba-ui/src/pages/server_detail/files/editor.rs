use asiba_core::{ActionRequest, ServerId};
use asiba_modules::files;
use egui::{RichText, TextEdit, Ui};

use super::state::Editor;
use crate::components::scroll;
use crate::pages::Action;
use crate::text;
use crate::theme::{GAP, Palette};

const MAX_HEIGHT: f32 = 380.0;

pub enum EditorOutcome {
    Keep,
    Close,
}

pub fn show(
    ui: &mut Ui,
    server: &ServerId,
    editor: &mut Editor,
) -> (EditorOutcome, Option<Action>) {
    let p = Palette::current(ui.ctx());
    let mut outcome = EditorOutcome::Keep;
    let mut action = None;
    ui.horizontal(|ui| {
        ui.label(RichText::new(&editor.path).monospace().strong());
        ui.label(
            RichText::new(format!("{} {}", editor.text.len(), text::FILES_BYTES))
                .monospace()
                .color(p.text_muted),
        );
        if editor.is_dirty() {
            ui.label(RichText::new(text::FILES_MODIFIED).color(p.warning));
        }
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            if ui.button(text::FILES_CLOSE).clicked() {
                outcome = EditorOutcome::Close;
            }
            if ui
                .add_enabled(editor.is_dirty(), egui::Button::new(text::FILES_REVERT))
                .clicked()
            {
                editor.text = editor.original.clone();
            }
            if ui
                .add_enabled(editor.is_dirty(), egui::Button::new(text::FILES_SAVE))
                .clicked()
            {
                action = Some(Action::AskPerform {
                    server: server.clone(),
                    spec: files::SPEC_WRITE,
                    request: ActionRequest::new(files::ACTION_WRITE, &editor.path)
                        .with_argument(editor.text.clone()),
                });
            }
        });
    });
    ui.add_space(GAP);
    scroll::vertical()
        .id_salt(("files-editor-scroll", editor.path.as_str()))
        .max_height(MAX_HEIGHT)
        .show(ui, |ui| {
            ui.add(
                TextEdit::multiline(&mut editor.text)
                    .code_editor()
                    .desired_width(f32::INFINITY)
                    .desired_rows(12),
            );
        });
    (outcome, action)
}
