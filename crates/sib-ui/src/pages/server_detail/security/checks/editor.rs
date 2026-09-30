use egui::{Grid, RichText, TextEdit, Ui};
use sib_core::{CheckExpect, CheckKind, CustomCheck};

use super::rows::{area_picker, cell_ui, weight_picker};
use crate::components::{chip_value, help};
use crate::text;
use crate::theme::{GAP, Palette, ROW_HEIGHT};

const LABEL_FIELD: f32 = 170.0;
const VALUE_FIELD: f32 = 520.0;
const TEXT_FIELD: f32 = 240.0;
const SCRIPT_ROWS: usize = 6;
const TEXT_ROWS: usize = 2;

pub fn show(ui: &mut Ui, check: &mut CustomCheck) {
    Grid::new(("check-editor", check.id.clone()))
        .num_columns(2)
        .spacing([GAP, GAP])
        .show(ui, |ui| {
            what_runs(ui, check);
            how_it_counts(ui, check);
            texts(ui, check);
        });
}

fn what_runs(ui: &mut Ui, check: &mut CustomCheck) {
    label(ui, text::RULE_NAME);
    ui.horizontal(|ui| {
        ui.add_sized(
            [VALUE_FIELD, ROW_HEIGHT],
            TextEdit::singleline(&mut check.name),
        );
        help(ui, text::CHECKS_HINT);
    });
    ui.end_row();
    label(ui, text::CHECK_RUNS);
    ui.horizontal_wrapped(|ui| {
        for kind in CheckKind::ALL {
            chip_value(ui, &mut check.kind, kind, kind_label(kind));
        }
    });
    ui.end_row();
    label(ui, source_label(check.kind));
    source_field(ui, check);
    ui.end_row();
}

fn how_it_counts(ui: &mut Ui, check: &mut CustomCheck) {
    let id = check.id.clone();
    label(ui, text::CHECK_EXPECT);
    ui.horizontal_wrapped(|ui| {
        for expect in CheckExpect::ALL {
            chip_value(ui, &mut check.expect, expect, expect_label(expect));
        }
        if check.expect.needs_text() {
            ui.add_sized(
                [TEXT_FIELD, ROW_HEIGHT],
                TextEdit::singleline(&mut check.expect_text).hint_text(text::CHECK_EXPECT_TEXT),
            );
        }
    });
    ui.end_row();
    label(ui, text::CHECK_AREA);
    area_picker(ui, &id, &mut check.area);
    ui.end_row();
    label(ui, text::CHECK_WEIGHT);
    weight_picker(ui, &id, &mut check.weight);
    ui.end_row();
    label(ui, text::CHECK_RUN_AS);
    ui.checkbox(&mut check.as_root, text::CHECK_AS_ROOT);
    ui.end_row();
}

fn texts(ui: &mut Ui, check: &mut CustomCheck) {
    label(ui, text::CHECK_DESCRIPTION);
    text_area(ui, &mut check.description);
    ui.end_row();
    label(ui, text::CHECK_ADVICE);
    text_area(ui, &mut check.advice);
    ui.end_row();
}

fn source_field(ui: &mut Ui, check: &mut CustomCheck) {
    if check.kind != CheckKind::Script {
        ui.add_sized(
            [VALUE_FIELD, ROW_HEIGHT],
            TextEdit::singleline(&mut check.source).hint_text("/opt/checks/disk.sh"),
        );
        return;
    }
    ui.add_sized(
        [VALUE_FIELD, ROW_HEIGHT * SCRIPT_ROWS as f32],
        TextEdit::multiline(&mut check.source)
            .code_editor()
            .desired_rows(SCRIPT_ROWS),
    );
}

fn text_area(ui: &mut Ui, value: &mut String) {
    ui.add_sized(
        [VALUE_FIELD, ROW_HEIGHT * TEXT_ROWS as f32],
        TextEdit::multiline(value).desired_rows(TEXT_ROWS),
    );
}

fn label(ui: &mut Ui, text: &str) {
    let p = Palette::current(ui.ctx());
    cell_ui(ui, LABEL_FIELD, |ui| {
        ui.label(RichText::new(text).color(p.text_secondary));
    });
}

fn source_label(kind: CheckKind) -> &'static str {
    match kind {
        CheckKind::Script => text::CHECK_SOURCE_SCRIPT,
        _ => text::CHECK_SOURCE_PATH,
    }
}

pub fn kind_label(kind: CheckKind) -> &'static str {
    match kind {
        CheckKind::Script => text::CHECK_KIND_SCRIPT,
        CheckKind::LocalFile => text::CHECK_KIND_LOCAL,
        CheckKind::RemoteFile => text::CHECK_KIND_REMOTE,
    }
}

pub fn expect_label(expect: CheckExpect) -> &'static str {
    match expect {
        CheckExpect::ExitZero => text::CHECK_EXPECT_EXIT,
        CheckExpect::Contains => text::CHECK_EXPECT_CONTAINS,
        CheckExpect::Missing => text::CHECK_EXPECT_MISSING,
    }
}
