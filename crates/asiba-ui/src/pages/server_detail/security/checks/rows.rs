use asiba_core::{Area, Weight};
use asiba_incidents::Pattern;
use egui::{ComboBox, Label, RichText, Ui};

use super::draft::Draft;
use crate::components::{Table, chip_value};
use crate::text;
use crate::theme::{Palette, ROW_HEIGHT};

const NAME_FIELD: f32 = 300.0;
const AREA_FIELD: f32 = 170.0;
const WEIGHT_FIELD: f32 = 110.0;
const ACTION_FIELD: f32 = 150.0;

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
    name_cell(ui, pattern.subject, None).on_hover_text(pattern.description);
    cell_ui(ui, AREA_FIELD, |ui| {
        ui.label(RichText::new(pattern.area.label()).color(p.text_secondary));
    });
    let mut weight = draft.weight_of(pattern.id, pattern.weight);
    if weight_picker(ui, pattern.id, &mut weight) {
        draft.set_weight(pattern.id, weight, pattern.weight);
    }
    let mut enabled = draft.is_enabled(pattern.id);
    if ui.checkbox(&mut enabled, "").changed() {
        draft.set_enabled(pattern.id, enabled);
    }
    let is_reset = actions(ui, |ui| {
        draft.is_overridden(pattern.id) && ui.small_button(text::RULE_RESET).clicked()
    });
    is_reset.then(|| Change::Reset(pattern.id.to_owned()))
}

fn custom_row(ui: &mut Ui, draft: &mut Draft, index: usize) -> Option<Change> {
    let check = draft.checks.get_mut(index)?;
    let id = check.id.clone();
    name_cell(ui, &check.name, Some(text::CHECK_OWN));
    area_picker(ui, &id, &mut check.area);
    weight_picker(ui, &id, &mut check.weight);
    ui.checkbox(&mut check.enabled, "");
    let mut change = None;
    actions(ui, |ui| {
        if ui.small_button(text::CHECK_EDIT).clicked() {
            change = Some(Change::Edit(id.clone()));
        }
        if ui.small_button(text::RULE_REMOVE).clicked() {
            change = Some(Change::Remove(id.clone()));
        }
    });
    change
}

fn name_cell(ui: &mut Ui, name: &str, mark: Option<&str>) -> egui::Response {
    let p = Palette::current(ui.ctx());
    cell_ui(ui, NAME_FIELD, |ui| {
        ui.add(Label::new(name).truncate());
        if let Some(mark) = mark {
            ui.label(RichText::new(mark).small().color(p.accent));
        }
    })
    .response
}

fn actions<R>(ui: &mut Ui, add_contents: impl FnOnce(&mut Ui) -> R) -> R {
    cell_ui(ui, ACTION_FIELD, add_contents).inner
}

pub fn cell_ui<R>(
    ui: &mut Ui,
    width: f32,
    add_contents: impl FnOnce(&mut Ui) -> R,
) -> egui::InnerResponse<R> {
    let rect = egui::Rect::from_min_size(ui.cursor().min, egui::vec2(width, ROW_HEIGHT));
    let scope = ui.scope_builder(egui::UiBuilder::new().max_rect(rect), |ui| {
        ui.horizontal(|ui| add_contents(ui)).inner
    });
    egui::InnerResponse::new(scope.inner, scope.response)
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
