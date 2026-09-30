use asiba_core::ServerState;
use asiba_modules::checks::{self, ChecksSnapshot};
use egui::{RichText, Ui};

use super::popup;
use crate::components::block_settings::{self, Popup};
use crate::components::{Table, badge, panel_with_controls, severity_color};
use crate::pages::Action;
use crate::pages::alert_rules::severity_label;
use crate::text;
use crate::theme::{GAP, Palette};

const KEY: &str = "server-checks";

pub fn show(ui: &mut Ui, server: &ServerState, state: &asiba_core::AppState) -> Option<Action> {
    panel_with_controls(
        ui,
        text::SEC_CHECKS_TITLE,
        |ui| block_settings::gear(ui, KEY),
        |ui| summary(ui, server),
    );
    ui.add_space(GAP);
    settings(ui, server, state)
}

fn settings(ui: &mut Ui, server: &ServerState, state: &asiba_core::AppState) -> Option<Action> {
    let popup_spec = Popup {
        key: KEY,
        title: text::SEC_CHECKS_TITLE,
    };
    let mut draft = popup::load(ui.ctx(), server);
    let action = block_settings::popup(ui, &popup_spec, |ui| {
        block_settings::scrolled(ui, |ui| popup::body(ui, &mut draft, server, state));
        popup::controls(ui, &mut draft, server)
    });
    let action = action?;
    popup::store(ui.ctx(), server, draft, action.is_some());
    action
}

fn summary(ui: &mut Ui, server: &ServerState) {
    let p = Palette::current(ui.ctx());
    if server.spec.checks.is_empty() {
        ui.label(RichText::new(text::SEC_CHECKS_EMPTY).color(p.text_muted));
        return;
    }
    let snapshot = server.data::<ChecksSnapshot>(checks::ID);
    let columns = [text::RULE_NAME, text::CHECK_OUTCOME, text::RULE_ENABLED];
    Table::new("server-checks-summary", &columns).show(ui, |ui| {
        for check in &server.spec.checks {
            ui.label(&check.name);
            outcome_label(ui, snapshot, &check.id, &p);
            badge(
                ui,
                severity_label(check.severity),
                severity_color(&p, check.severity),
            );
            ui.end_row();
        }
    });
}

fn outcome_label(ui: &mut Ui, snapshot: Option<&ChecksSnapshot>, id: &str, p: &Palette) {
    let Some(result) = snapshot.and_then(|s| s.find(id)) else {
        ui.label(RichText::new(text::CHECK_EMPTY).color(p.text_muted));
        return;
    };
    let (label, color) = match result.outcome {
        checks::CheckOutcome::Passed => (text::CHECK_PASSED, p.ok),
        checks::CheckOutcome::Failed => (text::CHECK_FAILED, p.critical),
        checks::CheckOutcome::Skipped => (text::CHECK_SKIPPED, p.text_muted),
    };
    badge(ui, label, color);
}
