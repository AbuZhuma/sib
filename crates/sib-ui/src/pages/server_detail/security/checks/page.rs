use egui::{Align, Button, Id, Layout, RichText, Ui};
use sib_core::{AppState, ServerState};

use super::draft::Draft;
use super::{editor, import, rows};
use crate::components::{help, panel, panel_plain};
use crate::pages::Action;
use crate::text;
use crate::theme::{GAP, Palette};

const DRAFT_KEY: &str = "server-checks-draft";

pub fn show(ui: &mut Ui, server: &ServerState, state: &AppState) -> Option<Action> {
    let mut draft: Draft = ui
        .ctx()
        .data(|d| d.get_temp(draft_id(server)))
        .unwrap_or_else(|| Draft::from_server(server));
    let action = header(ui, &mut draft, server);
    ui.add_space(GAP);
    rows::area_filter(ui, &mut draft);
    ui.add_space(GAP);
    rows::show(ui, &mut draft);
    ui.add_space(GAP);
    open_import(ui, &mut draft, server, state);
    open_editor(ui, &mut draft);
    store(ui.ctx(), server, draft, action.is_some());
    action
}

fn draft_id(server: &ServerState) -> Id {
    Id::new((DRAFT_KEY, server.spec.id.as_str()))
}

fn store(ctx: &egui::Context, server: &ServerState, draft: Draft, is_saved: bool) {
    let id = draft_id(server);
    if is_saved {
        ctx.data_mut(|d| d.remove::<Draft>(id));
        return;
    }
    ctx.data_mut(|d| d.insert_temp(id, draft));
}

fn header(ui: &mut Ui, draft: &mut Draft, server: &ServerState) -> Option<Action> {
    let p = Palette::current(ui.ctx());
    let mut action = None;
    ui.horizontal(|ui| {
        ui.heading(text::CHECKS_TITLE);
        if !draft.is_complete() {
            ui.label(RichText::new(text::CHECK_INCOMPLETE).color(p.warning));
        }
        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
            help(ui, text::CHECKS_HINT);
            action = apply_button(ui, draft, server);
            if ui.button(text::CHECK_IMPORT).clicked() {
                draft.is_importing = !draft.is_importing;
            }
            if ui.button(text::CHECK_ADD).clicked() {
                draft.add();
            }
        });
    });
    action
}

fn apply_button(ui: &mut Ui, draft: &Draft, server: &ServerState) -> Option<Action> {
    let is_ready = draft.is_dirty(server) && draft.is_complete();
    if !ui
        .add_enabled(is_ready, Button::new(text::BTN_APPLY))
        .clicked()
    {
        return None;
    }
    Some(Action::SaveServerChecks {
        server: server.spec.id.clone(),
        checks: draft.checks.clone(),
        overrides: draft.saved_overrides(),
    })
}

fn open_import(ui: &mut Ui, draft: &mut Draft, server: &ServerState, state: &AppState) {
    if !draft.is_importing {
        return;
    }
    panel(ui, text::CHECK_IMPORT_TITLE, |ui| {
        import::show(ui, state, &server.spec.id, draft);
    });
    ui.add_space(GAP);
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
}
