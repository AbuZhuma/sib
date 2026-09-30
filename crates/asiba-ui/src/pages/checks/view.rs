use asiba_config::AppConfig;
use egui::{Button, Id, RichText, Ui};

use super::draft::Draft;
use super::{editor, list};
use crate::components::{help, panel_plain};
use crate::pages::Action;
use crate::text;
use crate::theme::{GAP, Palette};

const DRAFT_KEY: &str = "custom-checks-draft";

pub fn show(ui: &mut Ui, config: &AppConfig) -> Option<Action> {
    let id = Id::new(DRAFT_KEY);
    let mut draft: Draft = ui
        .ctx()
        .data(|d| d.get_temp(id))
        .unwrap_or_else(|| Draft::from_config(config));
    list::show(ui, &mut draft);
    ui.add_space(GAP);
    open_editor(ui, &mut draft);
    let action = controls(ui, &mut draft, config);
    ui.ctx().data_mut(|d| d.insert_temp(id, draft));
    if action.is_some() {
        ui.ctx().data_mut(|d| d.remove::<Draft>(id));
    }
    action
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

fn controls(ui: &mut Ui, draft: &mut Draft, config: &AppConfig) -> Option<Action> {
    let p = Palette::current(ui.ctx());
    let is_dirty = draft.is_dirty(config);
    let is_complete = draft.is_complete();
    let mut action = None;
    ui.horizontal(|ui| {
        if ui.button(text::CHECK_ADD).clicked() {
            draft.add();
        }
        if ui
            .add_enabled(is_dirty && is_complete, Button::new(text::BTN_APPLY))
            .clicked()
        {
            action = Some(Action::SaveChecks(draft.checks.clone()));
        }
        help(ui, text::CHECKS_HINT);
        if !is_complete {
            ui.label(RichText::new(text::CHECK_INCOMPLETE).color(p.warning));
        }
    });
    action
}
