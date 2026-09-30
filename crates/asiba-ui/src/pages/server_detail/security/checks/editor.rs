use asiba_core::{CheckExpect, CheckKind, CustomCheck, Severity};
use egui::{RichText, TextEdit, Ui};

use crate::components::{chip_value, help};
use crate::text;
use crate::theme::{FIELD_WIDTH, GAP, Palette};

const SCRIPT_ROWS: usize = 6;
const TEXT_ROWS: usize = 2;

pub fn show(ui: &mut Ui, check: &mut CustomCheck) {
    let p = Palette::current(ui.ctx());
    ui.horizontal(|ui| {
        ui.label(RichText::new(text::RULE_NAME).color(p.text_secondary));
        ui.add(TextEdit::singleline(&mut check.name).desired_width(FIELD_WIDTH));
        help(ui, text::CHECKS_HINT);
    });
    ui.add_space(GAP);
    kind_picker(ui, check, &p);
    source_field(ui, check);
    ui.add_space(GAP);
    expectation(ui, check, &p);
    ui.add_space(GAP);
    outcome_row(ui, check, &p);
    ui.add_space(GAP);
    text_area(ui, text::CHECK_DESCRIPTION, &mut check.description, &p);
    text_area(ui, text::CHECK_ADVICE, &mut check.advice, &p);
}

fn kind_picker(ui: &mut Ui, check: &mut CustomCheck, p: &Palette) {
    ui.horizontal_wrapped(|ui| {
        ui.label(RichText::new(text::CHECK_RUNS).color(p.text_secondary));
        for kind in CheckKind::ALL {
            chip_value(ui, &mut check.kind, kind, kind_label(kind));
        }
    });
}

fn source_field(ui: &mut Ui, check: &mut CustomCheck) {
    let p = Palette::current(ui.ctx());
    if check.kind == CheckKind::Script {
        ui.label(RichText::new(text::CHECK_SOURCE_SCRIPT).color(p.text_secondary));
        ui.add(
            TextEdit::multiline(&mut check.source)
                .code_editor()
                .desired_rows(SCRIPT_ROWS)
                .desired_width(f32::INFINITY),
        );
        return;
    }
    ui.horizontal(|ui| {
        ui.label(RichText::new(text::CHECK_SOURCE_PATH).color(p.text_secondary));
        ui.add(
            TextEdit::singleline(&mut check.source)
                .hint_text("/opt/checks/disk.sh")
                .desired_width(FIELD_WIDTH),
        );
    });
}

fn expectation(ui: &mut Ui, check: &mut CustomCheck, p: &Palette) {
    ui.horizontal_wrapped(|ui| {
        ui.label(RichText::new(text::CHECK_EXPECT).color(p.text_secondary));
        for expect in CheckExpect::ALL {
            chip_value(ui, &mut check.expect, expect, expect_label(expect));
        }
        if check.expect.needs_text() {
            ui.add(
                TextEdit::singleline(&mut check.expect_text)
                    .hint_text(text::CHECK_EXPECT_TEXT)
                    .desired_width(FIELD_WIDTH / 2.0),
            );
        }
    });
}

fn outcome_row(ui: &mut Ui, check: &mut CustomCheck, p: &Palette) {
    ui.horizontal_wrapped(|ui| {
        ui.label(RichText::new(text::CHECK_SEVERITY).color(p.text_secondary));
        chip_value(
            ui,
            &mut check.severity,
            Severity::Warning,
            text::SEVERITY_WARNING,
        );
        chip_value(
            ui,
            &mut check.severity,
            Severity::Critical,
            text::SEVERITY_CRITICAL,
        );
        ui.add_space(GAP);
        ui.checkbox(&mut check.as_root, text::CHECK_AS_ROOT);
    });
}

fn text_area(ui: &mut Ui, label: &str, value: &mut String, p: &Palette) {
    ui.label(RichText::new(label).color(p.text_secondary));
    ui.add(
        TextEdit::multiline(value)
            .desired_rows(TEXT_ROWS)
            .desired_width(f32::INFINITY),
    );
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
