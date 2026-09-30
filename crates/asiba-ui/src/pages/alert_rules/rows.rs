use asiba_core::{AlertRule, Condition, Severity};
use egui::{ComboBox, DragValue, TextEdit, Ui};

use super::draft::Draft;
use crate::components::{Table, chip_value};
use crate::text;

const NAME_FIELD: f32 = 170.0;
const METRIC_FIELD: f32 = 190.0;
const SYMBOL_FIELD: f32 = 40.0;
const NUMBER_SPEED: f64 = 0.5;
const SECONDS_SPEED: f64 = 5.0;

enum Change {
    Reset(String),
    Remove(String),
}

pub fn rules_table(ui: &mut Ui, draft: &mut Draft) {
    let columns = [
        text::RULE_NAME,
        text::RULE_METRIC,
        text::RULE_CONDITION,
        text::RULE_THRESHOLD,
        text::RULE_FOR,
        text::RULE_SEVERITY,
        text::RULE_ENABLED,
        "",
    ];
    let defaults = asiba_alerts::builtin_rules();
    let mut change = None;
    Table::new("alert-rules", &columns).show(ui, |ui| {
        for rule in &mut draft.rules {
            fields(ui, rule);
            if let Some(next) = row_button(ui, rule, &defaults) {
                change = Some(next);
            }
            ui.end_row();
        }
    });
    match change {
        Some(Change::Reset(id)) => draft.reset(&id),
        Some(Change::Remove(id)) => draft.remove(&id),
        None => {}
    }
}

fn fields(ui: &mut Ui, rule: &mut AlertRule) {
    ui.add(
        TextEdit::singleline(&mut rule.name)
            .hint_text(text::RULE_NAME)
            .desired_width(NAME_FIELD),
    );
    ui.add(
        TextEdit::singleline(&mut rule.metric)
            .hint_text(text::RULE_METRIC)
            .desired_width(METRIC_FIELD),
    );
    condition_picker(ui, &rule.id, &mut rule.condition);
    ui.add(DragValue::new(&mut rule.threshold).speed(NUMBER_SPEED));
    ui.add(DragValue::new(&mut rule.for_secs).speed(SECONDS_SPEED));
    severity_picker(ui, &rule.id, &mut rule.severity);
    ui.checkbox(&mut rule.enabled, "");
}

fn row_button(ui: &mut Ui, rule: &AlertRule, defaults: &[AlertRule]) -> Option<Change> {
    if !rule.builtin {
        return ui
            .small_button(text::RULE_REMOVE)
            .clicked()
            .then(|| Change::Remove(rule.id.clone()));
    }
    if defaults.contains(rule) {
        ui.label("");
        return None;
    }
    ui.small_button(text::RULE_RESET)
        .clicked()
        .then(|| Change::Reset(rule.id.clone()))
}

fn condition_picker(ui: &mut Ui, id: &str, condition: &mut Condition) {
    ComboBox::from_id_salt(("rule-condition", id))
        .selected_text(condition.symbol())
        .width(SYMBOL_FIELD)
        .show_ui(ui, |ui| {
            chip_value(ui, condition, Condition::Above, Condition::Above.symbol());
            chip_value(ui, condition, Condition::Below, Condition::Below.symbol());
        });
}

fn severity_picker(ui: &mut Ui, id: &str, severity: &mut Severity) {
    ComboBox::from_id_salt(("rule-severity", id))
        .selected_text(severity_label(*severity))
        .show_ui(ui, |ui| {
            for level in [Severity::Info, Severity::Warning, Severity::Critical] {
                chip_value(ui, severity, level, severity_label(level));
            }
        });
}

pub fn severity_label(severity: Severity) -> &'static str {
    match severity {
        Severity::Info => text::SEVERITY_INFO,
        Severity::Warning => text::SEVERITY_WARNING,
        Severity::Critical => text::SEVERITY_CRITICAL,
    }
}
