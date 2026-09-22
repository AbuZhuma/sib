use asiba_config::AppConfig;
use asiba_core::{AlertRule, Condition, Severity};
use egui::{ComboBox, Id, RichText, TextEdit, Ui};

use super::Action;
use crate::components::{Table, badge, chip_value, severity_color};
use crate::text;
use crate::theme::{FIELD_WIDTH, GAP, Palette};

const DRAFT_KEY: &str = "alert-rule-draft";
const SHORT_FIELD: f32 = 90.0;

#[derive(Clone, Debug)]
struct Draft {
    name: String,
    metric: String,
    condition: Condition,
    threshold: String,
    for_secs: String,
    severity: Severity,
}

impl Default for Draft {
    fn default() -> Self {
        Self {
            name: String::new(),
            metric: String::new(),
            condition: Condition::Above,
            threshold: String::new(),
            for_secs: "60".to_owned(),
            severity: Severity::Warning,
        }
    }
}

impl Draft {
    fn build(&self) -> Option<AlertRule> {
        let name = self.name.trim();
        let metric = self.metric.trim();
        if name.is_empty() || metric.is_empty() {
            return None;
        }
        Some(AlertRule {
            id: format!("custom:{}", metric.replace(['/', ' '], "_")),
            name: name.to_owned(),
            metric: metric.to_owned(),
            condition: self.condition,
            threshold: self.threshold.trim().parse().ok()?,
            for_secs: self.for_secs.trim().parse().ok()?,
            severity: self.severity,
            builtin: false,
        })
    }
}

pub fn show(ui: &mut Ui, config: &AppConfig) -> Option<Action> {
    let mut notifications = config.desktop_notifications;
    let mut changed = ui
        .checkbox(&mut notifications, text::SETTINGS_NOTIFICATIONS)
        .changed();
    ui.add_space(GAP);
    let mut rules = config.alert_rules.clone();
    changed |= rules_table(ui, &mut rules);
    ui.add_space(GAP);
    changed |= add_form(ui, &mut rules);
    changed.then_some(Action::SaveAlertSettings {
        rules,
        desktop_notifications: notifications,
    })
}

fn rules_table(ui: &mut Ui, custom: &mut Vec<AlertRule>) -> bool {
    let p = Palette::current(ui.ctx());
    let mut removed = None;
    let columns = [
        text::RULE_NAME,
        text::RULE_CONDITION,
        text::RULE_SEVERITY,
        "",
        "",
    ];
    let merged = asiba_alerts::merge_rules(custom);
    Table::new("alert-rules", &columns).show(ui, |ui| {
        for rule in &merged {
            ui.label(&rule.name);
            ui.monospace(rule.summary());
            badge(
                ui,
                severity_label(rule.severity),
                severity_color(&p, rule.severity),
            );
            let origin = if rule.builtin {
                text::RULE_BUILTIN
            } else {
                text::RULE_CUSTOM
            };
            ui.label(RichText::new(origin).color(p.text_muted));
            if !rule.builtin && ui.small_button(text::RULE_REMOVE).clicked() {
                removed = Some(rule.id.clone());
            }
            ui.end_row();
        }
    });
    let Some(id) = removed else {
        return false;
    };
    custom.retain(|r| r.id != id);
    true
}

fn draft_fields(ui: &mut Ui, draft: &mut Draft) {
    let field = |ui: &mut Ui, value: &mut String, hint: &str, width: f32| {
        ui.add(
            TextEdit::singleline(value)
                .hint_text(hint)
                .desired_width(width),
        );
    };
    field(ui, &mut draft.name, text::RULE_NAME, FIELD_WIDTH / 2.0);
    field(ui, &mut draft.metric, text::RULE_METRIC, FIELD_WIDTH / 2.0);
    condition_picker(ui, &mut draft.condition);
    field(ui, &mut draft.threshold, text::RULE_THRESHOLD, SHORT_FIELD);
    field(ui, &mut draft.for_secs, text::RULE_FOR, SHORT_FIELD);
    severity_picker(ui, &mut draft.severity);
}

fn add_form(ui: &mut Ui, custom: &mut Vec<AlertRule>) -> bool {
    let p = Palette::current(ui.ctx());
    let id = Id::new(DRAFT_KEY);
    let mut draft: Draft = ui.ctx().data(|d| d.get_temp(id)).unwrap_or_default();
    ui.label(RichText::new(text::RULE_HINT).small().color(p.text_muted));
    ui.horizontal_wrapped(|ui| draft_fields(ui, &mut draft));
    let rule = draft.build();
    let added = ui
        .add_enabled(rule.is_some(), egui::Button::new(text::RULE_ADD))
        .clicked();
    if added && let Some(rule) = rule {
        custom.retain(|r| r.id != rule.id);
        custom.push(rule);
        draft = Draft::default();
    }
    ui.ctx().data_mut(|d| d.insert_temp(id, draft));
    added
}

fn condition_picker(ui: &mut Ui, condition: &mut Condition) {
    ComboBox::from_id_salt("rule-condition")
        .selected_text(condition.symbol())
        .width(SHORT_FIELD / 2.0)
        .show_ui(ui, |ui| {
            chip_value(ui, condition, Condition::Above, Condition::Above.symbol());
            chip_value(ui, condition, Condition::Below, Condition::Below.symbol());
        });
}

fn severity_picker(ui: &mut Ui, severity: &mut Severity) {
    ComboBox::from_id_salt("rule-severity")
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
