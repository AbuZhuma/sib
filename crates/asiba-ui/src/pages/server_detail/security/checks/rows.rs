use asiba_core::{Area, Weight};
use asiba_incidents::Pattern;
use egui::{ComboBox, RichText, Ui};

use super::draft::Draft;
use crate::components::{Table, chip_value};
use crate::text;
use crate::theme::Palette;

const AREA_FIELD: f32 = 150.0;
const WEIGHT_FIELD: f32 = 96.0;

enum Change {
    Edit(String),
    Remove(String),
    Reset(String),
}

enum Row {
    Builtin(&'static Pattern),
    Custom(usize),
}

pub fn area_filter(ui: &mut Ui, draft: &mut Draft) {
    let p = Palette::current(ui.ctx());
    ui.horizontal_wrapped(|ui| {
        ui.label(RichText::new(text::CHECK_AREA).color(p.text_secondary));
        chip_value(ui, &mut draft.area, None, text::CHECK_AREA_ALL);
        for area in Area::ALL {
            chip_value(ui, &mut draft.area, Some(area), area.label());
        }
    });
}

pub fn show(ui: &mut Ui, draft: &mut Draft) {
    let columns = [
        text::CHECK_NAME,
        text::CHECK_AREA,
        text::CHECK_WEIGHT,
        text::RULE_ENABLED,
        "",
        "",
    ];
    let rows = ordered(draft);
    let mut change = None;
    Table::new("audit-checks", &columns).show(ui, |ui| {
        for row in rows {
            let next = match row {
                Row::Builtin(pattern) => builtin_row(ui, draft, pattern),
                Row::Custom(index) => custom_row(ui, draft, index),
            };
            if next.is_some() {
                change = next;
            }
            ui.end_row();
        }
    });
    apply(draft, change);
}

fn apply(draft: &mut Draft, change: Option<Change>) {
    match change {
        Some(Change::Edit(id)) => draft.toggle_editing(&id),
        Some(Change::Remove(id)) => draft.remove(&id),
        Some(Change::Reset(id)) => draft.reset(&id),
        None => {}
    }
}

fn ordered(draft: &Draft) -> Vec<Row> {
    let mut rows: Vec<(Area, &str, Row)> = asiba_incidents::patterns()
        .filter(|pattern| draft.area.is_none_or(|area| area == pattern.area))
        .map(|pattern| (pattern.area, pattern.subject, Row::Builtin(pattern)))
        .collect();
    for (index, check) in draft.checks.iter().enumerate() {
        if draft.area.is_none_or(|area| area == check.area) {
            rows.push((check.area, check.name.as_str(), Row::Custom(index)));
        }
    }
    rows.sort_by_key(|(area, name, _)| (*area, *name));
    rows.into_iter().map(|(_, _, row)| row).collect()
}

fn builtin_row(ui: &mut Ui, draft: &mut Draft, pattern: &'static Pattern) -> Option<Change> {
    let p = Palette::current(ui.ctx());
    ui.label(pattern.subject).on_hover_text(pattern.description);
    ui.label(RichText::new(pattern.area.label()).color(p.text_secondary));
    let mut weight = draft.weight_of(pattern.id, pattern.weight);
    if weight_picker(ui, pattern.id, &mut weight) {
        draft.set_weight(pattern.id, weight, pattern.weight);
    }
    let mut enabled = draft.is_enabled(pattern.id);
    if ui.checkbox(&mut enabled, "").changed() {
        draft.set_enabled(pattern.id, enabled);
    }
    ui.label("");
    if !draft.is_overridden(pattern.id) {
        ui.label("");
        return None;
    }
    ui.small_button(text::RULE_RESET)
        .clicked()
        .then(|| Change::Reset(pattern.id.to_owned()))
}

fn custom_row(ui: &mut Ui, draft: &mut Draft, index: usize) -> Option<Change> {
    let p = Palette::current(ui.ctx());
    let check = draft.checks.get_mut(index)?;
    let id = check.id.clone();
    ui.horizontal(|ui| {
        ui.label(&check.name);
        ui.label(RichText::new(text::CHECK_OWN).small().color(p.accent));
    });
    area_picker(ui, &id, &mut check.area);
    weight_picker(ui, &id, &mut check.weight);
    ui.checkbox(&mut check.enabled, "");
    let is_edited = ui.small_button(text::CHECK_EDIT).clicked();
    let is_removed = ui.small_button(text::RULE_REMOVE).clicked();
    if is_removed {
        return Some(Change::Remove(id));
    }
    is_edited.then_some(Change::Edit(id))
}

pub fn area_picker(ui: &mut Ui, id: &str, area: &mut Area) {
    ComboBox::from_id_salt(("check-area", id))
        .selected_text(area.label())
        .width(AREA_FIELD)
        .show_ui(ui, |ui| {
            for value in Area::ALL {
                ui.selectable_value(area, value, value.label());
            }
        });
}

pub fn weight_picker(ui: &mut Ui, id: &str, weight: &mut Weight) -> bool {
    let mut changed = false;
    ComboBox::from_id_salt(("check-weight", id))
        .selected_text(weight.label())
        .width(WEIGHT_FIELD)
        .show_ui(ui, |ui| {
            for value in Weight::ALL {
                changed |= ui.selectable_value(weight, value, value.label()).changed();
            }
        });
    changed
}
