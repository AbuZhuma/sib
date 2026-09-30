use asiba_config::AppConfig;
use egui::{Button, Id, RichText, Ui};

use super::draft::Draft;
use super::rows;
use crate::components::help;
use crate::pages::Action;
use crate::text;
use crate::theme::{GAP, Palette};

const DRAFT_KEY: &str = "alert-rules-draft";

pub fn show(ui: &mut Ui, config: &AppConfig) -> Option<Action> {
    let id = Id::new(DRAFT_KEY);
    let mut draft: Draft = ui
        .ctx()
        .data(|d| d.get_temp(id))
        .unwrap_or_else(|| Draft::from_config(config));
    ui.checkbox(&mut draft.notifications, text::SETTINGS_NOTIFICATIONS);
    ui.add_space(GAP);
    rows::rules_table(ui, &mut draft);
    ui.add_space(GAP);
    let action = controls(ui, &mut draft, config);
    ui.ctx().data_mut(|d| d.insert_temp(id, draft));
    if action.is_some() {
        ui.ctx().data_mut(|d| d.remove::<Draft>(id));
    }
    action
}

fn controls(ui: &mut Ui, draft: &mut Draft, config: &AppConfig) -> Option<Action> {
    let p = Palette::current(ui.ctx());
    let is_dirty = *draft != Draft::from_config(config);
    let is_complete = draft.is_complete();
    let mut action = None;
    ui.horizontal(|ui| {
        if ui.button(text::RULE_ADD).clicked() {
            draft.add();
        }
        if ui
            .add_enabled(is_dirty && is_complete, Button::new(text::BTN_APPLY))
            .clicked()
        {
            action = Some(Action::SaveAlertSettings {
                rules: draft.saved_rules(),
                desktop_notifications: draft.notifications,
            });
        }
        help(ui, text::RULE_HINT);
        if !is_complete {
            ui.label(RichText::new(text::RULE_INCOMPLETE).color(p.warning));
        }
    });
    action
}
