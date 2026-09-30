use asiba_core::{CheckExpect, CustomCheck};
use egui::{RichText, Ui};

use super::draft::Draft;
use super::editor;
use crate::components::{Table, badge, severity_color};
use crate::pages::alert_rules::severity_label;
use crate::text;
use crate::theme::Palette;

const SOURCE_LIMIT: usize = 60;

enum Change {
    Edit(String),
    Remove(String),
}

pub fn show(ui: &mut Ui, draft: &mut Draft) {
    let p = Palette::current(ui.ctx());
    let columns = [
        text::RULE_NAME,
        text::CHECK_RUNS,
        text::CHECK_CONDITION,
        text::RULE_SEVERITY,
        text::RULE_ENABLED,
        "",
        "",
    ];
    let mut change = None;
    Table::new("custom-checks", &columns).show(ui, |ui| {
        for check in &mut draft.checks {
            row(ui, check, &p);
            if ui.small_button(text::CHECK_EDIT).clicked() {
                change = Some(Change::Edit(check.id.clone()));
            }
            if ui.small_button(text::RULE_REMOVE).clicked() {
                change = Some(Change::Remove(check.id.clone()));
            }
            ui.end_row();
        }
    });
    match change {
        Some(Change::Edit(id)) => draft.toggle_editing(&id),
        Some(Change::Remove(id)) => draft.remove(&id),
        None => {}
    }
}

fn row(ui: &mut Ui, check: &mut CustomCheck, p: &Palette) {
    ui.label(&check.name);
    ui.monospace(RichText::new(source_line(check)).color(p.text_secondary));
    ui.label(RichText::new(condition_line(check)).color(p.text_secondary));
    badge(
        ui,
        severity_label(check.severity),
        severity_color(p, check.severity),
    );
    ui.checkbox(&mut check.enabled, "");
}

fn source_line(check: &CustomCheck) -> String {
    let source: String = check
        .source
        .trim()
        .lines()
        .next()
        .unwrap_or_default()
        .chars()
        .take(SOURCE_LIMIT)
        .collect();
    match check.as_root {
        true => format!("root: {source}"),
        false => source,
    }
}

fn condition_line(check: &CustomCheck) -> String {
    let label = editor::expect_label(check.expect);
    if check.expect == CheckExpect::ExitZero {
        return label.to_owned();
    }
    format!("{label} «{}»", check.expect_text.trim())
}
