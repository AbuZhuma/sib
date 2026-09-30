use asiba_core::{ModuleId, ServerState};
use asiba_modules::checks::{self, CheckOutcome, CheckResult, ChecksSnapshot};
use egui::{RichText, Ui};

use super::{ModuleView, Tab, ViewAction, ViewShared};
use crate::components::{Table, badge, panel_plain};
use crate::format;
use crate::text;
use crate::theme::{GAP, Palette};

pub struct ChecksView;

impl ModuleView for ChecksView {
    fn id(&self) -> ModuleId {
        checks::ID
    }

    fn title(&self) -> &'static str {
        text::MODULE_CHECKS
    }

    fn tab(&self) -> Tab {
        Tab::Checks
    }

    fn has_content(&self, server: &ServerState) -> bool {
        server
            .data::<ChecksSnapshot>(checks::ID)
            .is_some_and(|snapshot| !snapshot.results.is_empty())
    }

    fn summary(&self, ui: &mut Ui, server: &ServerState, _shared: &ViewShared) {
        let p = Palette::current(ui.ctx());
        let Some(snapshot) = server.data::<ChecksSnapshot>(checks::ID) else {
            return;
        };
        if snapshot.results.is_empty() {
            ui.label(RichText::new(text::CHECK_EMPTY).color(p.text_muted));
            return;
        }
        let columns = [text::RULE_NAME, text::CHECK_OUTCOME, text::COL_TIME];
        Table::new("checks-summary", &columns).show(ui, |ui| {
            for result in &snapshot.results {
                ui.label(&result.check.name);
                outcome_badge(ui, result, &p);
                ui.monospace(format::clock(result.at));
                ui.end_row();
            }
        });
    }

    fn page(&self, ui: &mut Ui, server: &ServerState, _shared: &ViewShared) -> Option<ViewAction> {
        let snapshot = server.data::<ChecksSnapshot>(checks::ID)?;
        for result in &snapshot.results {
            panel_plain(ui, |ui| card(ui, result));
            ui.add_space(GAP);
        }
        None
    }
}

fn card(ui: &mut Ui, result: &CheckResult) {
    let p = Palette::current(ui.ctx());
    ui.horizontal(|ui| {
        ui.label(RichText::new(&result.check.name).strong());
        outcome_badge(ui, result, &p);
        ui.label(
            RichText::new(format!("{} {}", text::CHECK_EXIT_CODE, result.exit_code))
                .small()
                .color(p.text_muted),
        );
        ui.monospace(RichText::new(format::clock(result.at)).color(p.text_muted));
    });
    ui.label(
        RichText::new(checks::metric_key(&result.check))
            .small()
            .color(p.text_muted),
    );
    if result.output.trim().is_empty() {
        ui.label(RichText::new(text::CHECK_NO_OUTPUT).color(p.text_muted));
        return;
    }
    ui.add_space(GAP);
    ui.monospace(RichText::new(&result.output).color(p.text_secondary));
}

fn outcome_badge(ui: &mut Ui, result: &CheckResult, p: &Palette) {
    let (label, color) = match result.outcome {
        CheckOutcome::Passed => (text::CHECK_PASSED, p.ok),
        CheckOutcome::Failed => (text::CHECK_FAILED, p.critical),
        CheckOutcome::Skipped => (text::CHECK_SKIPPED, p.text_muted),
    };
    badge(ui, label, color);
}
