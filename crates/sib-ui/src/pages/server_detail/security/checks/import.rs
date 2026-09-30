use egui::{RichText, Ui};
use sib_core::{AppState, CustomCheck, ServerId};

use super::draft::Draft;
use crate::components::Table;
use crate::text;
use crate::theme::Palette;

const SOURCE_LIMIT: usize = 50;

pub fn show(ui: &mut Ui, state: &AppState, current: &ServerId, draft: &mut Draft) {
    let p = Palette::current(ui.ctx());
    let available = elsewhere(state, current);
    if available.is_empty() {
        ui.label(RichText::new(text::CHECK_IMPORT_EMPTY).color(p.text_muted));
        return;
    }
    let columns = [
        text::CHECK_IMPORT_FROM,
        text::RULE_NAME,
        text::CHECK_RUNS,
        "",
    ];
    let mut taken = None;
    Table::new("checks-import", &columns).show(ui, |ui| {
        for (server, check) in &available {
            ui.label(RichText::new(server.as_str()).color(p.text_secondary));
            ui.label(&check.name);
            ui.monospace(RichText::new(source_line(check)).color(p.text_muted));
            if ui.small_button(text::CHECK_IMPORT_TAKE).clicked() {
                taken = Some((*check).clone());
            }
            ui.end_row();
        }
    });
    if let Some(check) = taken {
        draft.import(&check);
    }
}

fn elsewhere<'a>(state: &'a AppState, current: &ServerId) -> Vec<(&'a ServerId, &'a CustomCheck)> {
    state
        .servers
        .iter()
        .filter(|(id, _)| *id != current)
        .flat_map(|(id, server)| server.spec.checks.iter().map(move |check| (id, check)))
        .collect()
}

fn source_line(check: &CustomCheck) -> String {
    check
        .source
        .trim()
        .lines()
        .next()
        .unwrap_or_default()
        .chars()
        .take(SOURCE_LIMIT)
        .collect()
}
