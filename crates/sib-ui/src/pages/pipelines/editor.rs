use egui::{Align, Grid, Layout, RichText, TextEdit, Ui};
use sib_core::{Step, StepTarget, Variable};

use super::draft::Draft;
use crate::components::{Table, chip_value, help, panel_plain, section_label};
use crate::text;
use crate::theme::{GAP, GAP_SMALL, Palette, ROW_HEIGHT};

const NAME_FIELD: f32 = 400.0;
const DESCRIPTION_FIELD: f32 = 640.0;
const DESCRIPTION_ROWS: usize = 2;
const LABEL_COLUMN: f32 = 110.0;
const VAR_NAME_FIELD: f32 = 150.0;
const VAR_DEFAULT_FIELD: f32 = 240.0;
const VAR_FIXED_COLUMNS: f32 = 144.0;
const VAR_DESCRIPTION_MIN: f32 = 150.0;
const VAR_DESCRIPTION_MAX: f32 = 360.0;
const STEP_NAME_FIELD: f32 = 320.0;
const TIMEOUT_FIELD: f32 = 60.0;
const COMMAND_ROWS: usize = 6;

pub fn show(ui: &mut Ui, draft: &mut Draft) {
    let p = Palette::current(ui.ctx());
    let width = ui.available_width();
    header(ui, draft, width);
    ui.add_space(GAP);
    titled(ui, text::PIPELINE_VARIABLES, text::PIPELINE_VARIABLES_HINT);
    variables(ui, draft, width);
    ui.add_space(GAP);
    titled(ui, text::PIPELINE_STEPS, text::PIPELINE_STEPS_HINT);
    steps(ui, draft);
    if let Some(error) = &draft.error {
        ui.add_space(GAP);
        ui.label(RichText::new(error).color(p.critical));
    }
}

fn titled(ui: &mut Ui, title: &str, hint: &str) {
    ui.horizontal(|ui| {
        section_label(ui, title);
        help(ui, hint);
    });
    ui.add_space(GAP_SMALL);
}

fn header(ui: &mut Ui, draft: &mut Draft, width: f32) {
    let name_field = NAME_FIELD.min(width - LABEL_COLUMN);
    let description_field = DESCRIPTION_FIELD.min(width - LABEL_COLUMN);
    Grid::new("pipeline-editor-header")
        .num_columns(2)
        .spacing([GAP, GAP])
        .show(ui, |ui| {
            label(ui, text::PIPELINE_NAME);
            if field(ui, name_field, &mut draft.pipeline.name, "") {
                draft.sync_id();
            }
            ui.end_row();
            label(ui, text::PIPELINE_ID);
            ui.horizontal(|ui| {
                if field(
                    ui,
                    name_field,
                    &mut draft.pipeline.id,
                    text::PIPELINE_ID_HINT,
                ) {
                    draft.id_is_manual = true;
                }
                help(ui, text::PIPELINE_ID_HINT);
            });
            ui.end_row();
            label(ui, text::PIPELINE_DESCRIPTION);
            ui.add_sized(
                [description_field, ROW_HEIGHT * DESCRIPTION_ROWS as f32],
                TextEdit::multiline(&mut draft.pipeline.description)
                    .desired_rows(DESCRIPTION_ROWS)
                    .hint_text(text::PIPELINE_DESCRIPTION_HINT),
            );
            ui.end_row();
        });
}

fn label(ui: &mut Ui, text: &str) {
    let p = Palette::current(ui.ctx());
    ui.label(RichText::new(text).color(p.text_secondary));
}

fn field(ui: &mut Ui, width: f32, value: &mut String, hint: &str) -> bool {
    ui.add_sized(
        [width, ROW_HEIGHT],
        TextEdit::singleline(value).hint_text(hint),
    )
    .changed()
}

fn variables(ui: &mut Ui, draft: &mut Draft, width: f32) {
    let description_field = (width - VAR_NAME_FIELD - VAR_DEFAULT_FIELD - VAR_FIXED_COLUMNS)
        .clamp(VAR_DESCRIPTION_MIN, VAR_DESCRIPTION_MAX);
    let columns = [
        text::VAR_NAME,
        text::VAR_DEFAULT,
        text::VAR_SECRET,
        text::VAR_DESCRIPTION,
        "",
    ];
    let mut remove = None;
    Table::new("pipeline-variables", &columns).show(ui, |ui| {
        for (index, variable) in draft.pipeline.variables.iter_mut().enumerate() {
            if variable_row(ui, variable, description_field) {
                remove = Some(index);
            }
            ui.end_row();
        }
    });
    if let Some(index) = remove {
        draft.pipeline.variables.remove(index);
    }
    ui.add_space(GAP_SMALL);
    if ui.button(text::BTN_ADD_VARIABLE).clicked() {
        draft.add_variable();
    }
}

fn variable_row(ui: &mut Ui, variable: &mut Variable, description_field: f32) -> bool {
    field(ui, VAR_NAME_FIELD, &mut variable.name, text::VAR_NAME_HINT);
    ui.add_sized(
        [VAR_DEFAULT_FIELD, ROW_HEIGHT],
        TextEdit::singleline(&mut variable.default)
            .password(variable.secret)
            .hint_text(text::VAR_DEFAULT_HINT),
    );
    ui.checkbox(&mut variable.secret, "")
        .on_hover_text(text::VAR_SECRET_HINT);
    field(
        ui,
        description_field,
        &mut variable.description,
        text::VAR_DESCRIPTION_HINT,
    );
    ui.small_button(text::BTN_REMOVE)
        .on_hover_text(text::BTN_REMOVE_HINT)
        .clicked()
}

fn steps(ui: &mut Ui, draft: &mut Draft) {
    let count = draft.pipeline.steps.len();
    let mut command = None;
    for index in 0..count {
        if let Some(next) = step(ui, index, count, &mut draft.pipeline.steps[index]) {
            command = Some(next);
        }
        ui.add_space(GAP);
    }
    match command {
        Some(StepCommand::Remove(index)) => {
            draft.pipeline.steps.remove(index);
        }
        Some(StepCommand::Move(index, delta)) => draft.move_step(index, delta),
        None => {}
    }
    if ui.button(text::BTN_ADD_STEP).clicked() {
        draft.add_step();
    }
}

enum StepCommand {
    Remove(usize),
    Move(usize, isize),
}

fn step(ui: &mut Ui, index: usize, count: usize, step: &mut Step) -> Option<StepCommand> {
    let mut command = None;
    panel_plain(ui, |ui| {
        if let Some(next) = step_header(ui, index, count, &mut step.name) {
            command = Some(next);
        }
        ui.add_space(GAP_SMALL);
        step_options(ui, step);
        ui.add_space(GAP_SMALL);
        ui.add(
            TextEdit::multiline(&mut step.command)
                .code_editor()
                .desired_width(f32::INFINITY)
                .desired_rows(COMMAND_ROWS)
                .hint_text(text::STEP_COMMAND_HINT),
        );
    });
    command
}

fn step_header(ui: &mut Ui, index: usize, count: usize, name: &mut String) -> Option<StepCommand> {
    let p = Palette::current(ui.ctx());
    let mut command = None;
    ui.horizontal(|ui| {
        ui.label(
            RichText::new(format!("{} {}", text::STEP_NAME, index + 1))
                .strong()
                .color(p.text_secondary),
        );
        field(ui, STEP_NAME_FIELD, name, text::STEP_NAME_HINT);
        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
            if move_button(ui, text::BTN_REMOVE, count > 1, text::BTN_REMOVE_HINT) {
                command = Some(StepCommand::Remove(index));
            }
            if move_button(ui, text::BTN_DOWN, index + 1 < count, text::BTN_DOWN_HINT) {
                command = Some(StepCommand::Move(index, 1));
            }
            if move_button(ui, text::BTN_UP, index > 0, text::BTN_UP_HINT) {
                command = Some(StepCommand::Move(index, -1));
            }
        });
    });
    command
}

fn move_button(ui: &mut Ui, label: &str, enabled: bool, hint: &str) -> bool {
    ui.add_enabled(enabled, egui::Button::new(label))
        .on_hover_text(hint)
        .clicked()
}

fn step_options(ui: &mut Ui, step: &mut Step) {
    ui.horizontal_wrapped(|ui| {
        chip_value(
            ui,
            &mut step.target,
            StepTarget::Server,
            text::STEP_ON_SERVER,
        );
        chip_value(ui, &mut step.target, StepTarget::Local, text::STEP_LOCAL);
        ui.add_space(GAP);
        ui.add_enabled_ui(step.target == StepTarget::Server, |ui| {
            ui.checkbox(&mut step.as_root, text::STEP_AS_ROOT);
        });
        ui.checkbox(&mut step.continue_on_error, text::STEP_CONTINUE);
        ui.add_space(GAP);
        label(ui, text::STEP_TIMEOUT);
        timeout_field(ui, &mut step.timeout_secs);
        help(ui, text::STEP_TIMEOUT_HINT);
    });
}

fn timeout_field(ui: &mut Ui, value: &mut u64) {
    let mut raw = value.to_string();
    if ui
        .add_sized([TIMEOUT_FIELD, ROW_HEIGHT], TextEdit::singleline(&mut raw))
        .changed()
    {
        let digits: String = raw.chars().filter(char::is_ascii_digit).collect();
        *value = digits.parse().unwrap_or(0);
    }
}
