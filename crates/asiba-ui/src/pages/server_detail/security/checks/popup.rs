use asiba_core::{AppState, ServerState};
use egui::{Button, Id, RichText, Ui};

use super::draft::Draft;
use super::{editor, import, rows};
use crate::components::{help, panel_plain};
use crate::pages::Action;
use crate::text;
use crate::theme::{GAP, Palette};

const DRAFT_KEY: &str = "server-checks-draft";

pub fn load(ctx: &egui::Context, server: &ServerState) -> Draft {
    ctx.data(|d| d.get_temp(draft_id(server)))
        .unwrap_or_else(|| Draft::from_server(server))
}

pub fn store(ctx: &egui::Context, server: &ServerState, draft: Draft, is_saved: bool) {
    let id = draft_id(server);
    if is_saved {
        ctx.data_mut(|d| d.remove::<Draft>(id));
        return;
    }
    ctx.data_mut(|d| d.insert_temp(id, draft));
}

pub fn body(ui: &mut Ui, draft: &mut Draft, server: &ServerState, state: &AppState) {
    if draft.is_importing {
        panel_plain(ui, |ui| {
            import::show(ui, state, &server.spec.id, draft);
        });
        ui.add_space(GAP);
    }
    rows::area_filter(ui, draft);
    ui.add_space(GAP);
    rows::show(ui, draft);
    ui.add_space(GAP);
    open_editor(ui, draft);
}

fn draft_id(server: &ServerState) -> Id {
    Id::new((DRAFT_KEY, server.spec.id.as_str()))
}

fn open_editor(ui: &mut Ui, draft: &mut Draft) {
    let Some(id) = draft.editing.clone() else {
        return;
    };
    let Some(check) = draft.checks.iter_mut().find(|check| check.id == id) else {
        draft.editing = None;
        return;
    };
    panel_plain(ui, |ui| editor::show(ui, check));
    ui.add_space(GAP);
    if ui.button(text::CHECK_EDIT_DONE).clicked() {
        draft.editing = None;
    }
    ui.add_space(GAP);
}

pub fn controls(ui: &mut Ui, draft: &mut Draft, server: &ServerState) -> Option<Action> {
    let p = Palette::current(ui.ctx());
    let is_dirty = draft.is_dirty(server);
    let is_complete = draft.is_complete();
    let mut action = None;
    ui.horizontal(|ui| {
        if ui.button(text::CHECK_ADD).clicked() {
            draft.add();
        }
        if ui.button(text::CHECK_IMPORT).clicked() {
            draft.is_importing = !draft.is_importing;
        }
        if ui
            .add_enabled(is_dirty && is_complete, Button::new(text::BTN_APPLY))
            .clicked()
        {
            action = Some(Action::SaveServerChecks {
                server: server.spec.id.clone(),
                checks: draft.checks.clone(),
                overrides: draft.saved_overrides(),
            });
        }
        help(ui, text::CHECKS_HINT);
        if !is_complete {
            ui.label(RichText::new(text::CHECK_INCOMPLETE).color(p.warning));
        }
    });
    action
}
