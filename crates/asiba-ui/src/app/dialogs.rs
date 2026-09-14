use asiba_core::ServerId;
use asiba_engine::Command;

use super::AsibaApp;
use crate::pages::Page;
use crate::text;

pub struct DeleteDialog {
    pub id: ServerId,
    pub typed: String,
}

impl AsibaApp {
    pub(super) fn delete_modal(&mut self, ctx: &egui::Context) {
        let Some(dialog) = &mut self.delete_dialog else {
            return;
        };
        let mut close = false;
        let mut confirmed = None;
        egui::Modal::new(egui::Id::new("delete-server")).show(ctx, |ui| {
            ui.set_width(360.0);
            ui.heading(format!("{} {}", text::BTN_DELETE, dialog.id));
            ui.label(text::DETAIL_DELETE_PROMPT);
            ui.text_edit_singleline(&mut dialog.typed);
            ui.horizontal(|ui| {
                let matches = dialog.typed.trim() == dialog.id.as_str();
                if ui
                    .add_enabled(matches, egui::Button::new(text::BTN_CONFIRM_DELETE))
                    .clicked()
                {
                    confirmed = Some(dialog.id.clone());
                }
                if ui.button(text::BTN_CANCEL).clicked() {
                    close = true;
                }
            });
        });
        if let Some(id) = confirmed {
            self.engine.send(Command::RemoveServer(id));
            self.page = Page::Servers;
            close = true;
        }
        if close {
            self.delete_dialog = None;
        }
    }
}
