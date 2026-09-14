use asiba_core::AppState;
use egui::{RichText, Ui};

use crate::components::{Table, badge};
use crate::format;
use crate::text;
use crate::theme::Palette;

const SHOWN: usize = 100;

pub fn show(ui: &mut Ui, state: &AppState) {
    let p = Palette::current(ui.ctx());
    if state.actions.is_empty() {
        ui.label(RichText::new(text::JOURNAL_EMPTY).color(p.text_muted));
        return;
    }
    let columns = [
        text::COL_TIME,
        text::CONFIRM_SERVER,
        text::JOURNAL_ACTION,
        text::CONFIRM_TARGET,
        "",
        text::JOURNAL_RESULT,
    ];
    Table::new("action-journal", &columns).show(ui, |ui| {
        for record in state.actions.iter().take(SHOWN) {
            ui.monospace(format::date_time(record.at));
            ui.monospace(record.server.as_str());
            ui.monospace(format!("{}/{}", record.module, record.kind));
            let target = match &record.argument {
                Some(argument) if !argument.is_empty() => {
                    format!("{} ({argument})", record.target)
                }
                _ => record.target.clone(),
            };
            ui.monospace(target);
            if record.is_success {
                badge(ui, text::ACTION_DONE, p.ok);
            } else {
                badge(ui, text::ACTION_FAILED, p.critical);
            }
            ui.label(RichText::new(&record.message).color(p.text_secondary));
            ui.end_row();
        }
    });
}
