use std::collections::BTreeMap;

use egui::{Align, ComboBox, Grid, Id, Layout, RichText, TextEdit, Ui};
use sib_core::{
    Pipeline, PipelineBinding, PipelineRun, RunStatus, ServerState, StepRun, StepStatus,
};

use super::{Action, DetailContext};
use crate::components::{badge, help, panel, panel_with_controls, scroll};
use crate::format;
use crate::text;
use crate::theme::{FIELD_WIDTH, GAP, Palette, ROW_HEIGHT};

const BINDINGS_KEY: &str = "server-pipelines-draft";
const SELECTED_RUN_KEY: &str = "server-pipelines-run";
const SELECTED_STEP_KEY: &str = "server-pipelines-step";
const LOG_HEIGHT: f32 = 320.0;
const RUNS_SHOWN: usize = 30;

#[derive(Clone)]
struct BindingsDraft {
    bindings: Vec<PipelineBinding>,
    expanded: Option<String>,
}

impl BindingsDraft {
    fn from_server(server: &ServerState) -> Self {
        Self {
            bindings: server.spec.pipelines.clone(),
            expanded: None,
        }
    }

    fn is_dirty(&self, server: &ServerState) -> bool {
        self.bindings != server.spec.pipelines
    }
}

pub fn show(ui: &mut Ui, ctx: &DetailContext<'_>) -> Option<Action> {
    let server = ctx.server;
    let draft_id = Id::new((BINDINGS_KEY, server.spec.id.as_str()));
    let mut draft: BindingsDraft = ui
        .ctx()
        .data(|d| d.get_temp(draft_id))
        .unwrap_or_else(|| BindingsDraft::from_server(server));
    let mut action = bindings_panel(ui, ctx, &mut draft);
    ui.add_space(GAP);
    if let Some(next) = runs_panel(ui, ctx) {
        action = Some(next);
    }
    let is_saved = matches!(action, Some(Action::SaveServerPipelines { .. }));
    ui.ctx().data_mut(|d| {
        if is_saved {
            d.remove::<BindingsDraft>(draft_id);
        } else {
            d.insert_temp(draft_id, draft);
        }
    });
    action
}

fn bindings_panel(
    ui: &mut Ui,
    ctx: &DetailContext<'_>,
    draft: &mut BindingsDraft,
) -> Option<Action> {
    let p = Palette::current(ui.ctx());
    let server = ctx.server;
    let mut action = None;
    let is_ready = draft.is_dirty(server);
    let mut apply_clicked = false;
    panel_with_controls(
        ui,
        text::SERVER_PIPELINES,
        |ui| {
            help(ui, text::SERVER_PIPELINES_HINT);
            apply_clicked = ui
                .add_enabled(is_ready, egui::Button::new(text::BTN_APPLY))
                .clicked();
        },
        |ui| {
            if draft.bindings.is_empty() {
                ui.label(RichText::new(text::SERVER_PIPELINES_EMPTY).color(p.text_muted));
            }
            let mut detach = None;
            for index in 0..draft.bindings.len() {
                if let Some(next) = binding_row(ui, ctx, draft, index, &mut detach) {
                    action = Some(next);
                }
            }
            if let Some(index) = detach {
                draft.bindings.remove(index);
            }
            ui.add_space(GAP);
            attach_picker(ui, ctx.prototypes, draft);
        },
    );
    if apply_clicked {
        action = Some(Action::SaveServerPipelines {
            server: server.spec.id.clone(),
            bindings: draft.bindings.clone(),
        });
    }
    action
}

fn binding_row(
    ui: &mut Ui,
    ctx: &DetailContext<'_>,
    draft: &mut BindingsDraft,
    index: usize,
    detach: &mut Option<usize>,
) -> Option<Action> {
    let p = Palette::current(ui.ctx());
    let server = ctx.server;
    let pipeline_id = draft.bindings[index].pipeline.clone();
    let prototype = ctx.prototypes.iter().find(|p| p.id == pipeline_id);
    let mut action = None;
    ui.horizontal(|ui| {
        match prototype {
            Some(prototype) => {
                ui.label(RichText::new(&prototype.name).strong());
                ui.label(
                    RichText::new(format!(
                        "{} {}",
                        prototype.steps.len(),
                        text::RUN_DIALOG_STEPS
                    ))
                    .color(p.text_secondary),
                );
            }
            None => {
                ui.label(RichText::new(&pipeline_id).monospace());
                ui.label(RichText::new(text::MISSING_PIPELINE).color(p.warning));
            }
        }
        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
            if ui.small_button(text::DETACH_PIPELINE).clicked() {
                *detach = Some(index);
            }
            if let Some(prototype) = prototype {
                if !prototype.variables.is_empty() {
                    let is_open = draft.expanded.as_deref() == Some(pipeline_id.as_str());
                    if ui.selectable_label(is_open, text::BTN_VALUES).clicked() {
                        draft.expanded = (!is_open).then(|| pipeline_id.clone());
                    }
                }
                let is_saved = !draft.is_dirty(server);
                if ui
                    .add_enabled(is_saved, egui::Button::new(text::BTN_RUN).small())
                    .clicked()
                {
                    action = Some(Action::AskRunPipeline {
                        server: server.spec.id.clone(),
                        pipeline_id: pipeline_id.clone(),
                    });
                }
            }
        });
    });
    if let Some(prototype) = prototype
        && draft.expanded.as_deref() == Some(pipeline_id.as_str())
    {
        values_grid(ui, prototype, &mut draft.bindings[index].values);
    }
    action
}

fn values_grid(ui: &mut Ui, prototype: &Pipeline, values: &mut BTreeMap<String, String>) {
    let p = Palette::current(ui.ctx());
    Grid::new(("binding-values", prototype.id.as_str()))
        .num_columns(3)
        .spacing([GAP, GAP])
        .show(ui, |ui| {
            for variable in prototype.variables.iter().filter(|v| !v.secret) {
                ui.label(RichText::new(&variable.name).monospace());
                let value = values.entry(variable.name.clone()).or_default();
                ui.add_sized(
                    [FIELD_WIDTH, ROW_HEIGHT],
                    TextEdit::singleline(value).hint_text(&variable.default),
                );
                ui.label(RichText::new(&variable.description).color(p.text_muted));
                ui.end_row();
            }
        });
    ui.add_space(GAP);
}

fn attach_picker(ui: &mut Ui, prototypes: &[Pipeline], draft: &mut BindingsDraft) {
    let free: Vec<&Pipeline> = prototypes
        .iter()
        .filter(|p| !draft.bindings.iter().any(|b| b.pipeline == p.id))
        .collect();
    if free.is_empty() {
        return;
    }
    ComboBox::from_id_salt("attach-pipeline")
        .selected_text(text::ATTACH_PIPELINE)
        .show_ui(ui, |ui| {
            for prototype in free {
                if ui.selectable_label(false, &prototype.name).clicked() {
                    draft.bindings.push(PipelineBinding {
                        pipeline: prototype.id.clone(),
                        values: BTreeMap::new(),
                    });
                }
            }
        });
}

fn runs_panel(ui: &mut Ui, ctx: &DetailContext<'_>) -> Option<Action> {
    let p = Palette::current(ui.ctx());
    let server = ctx.server;
    let runs: Vec<&PipelineRun> = ctx
        .state
        .pipeline_runs_for(&server.spec.id)
        .take(RUNS_SHOWN)
        .collect();
    let mut action = None;
    panel(ui, text::RUNS_TITLE, |ui| {
        if runs.is_empty() {
            ui.label(RichText::new(text::RUNS_EMPTY).color(p.text_muted));
            return;
        }
        let selected_id = Id::new((SELECTED_RUN_KEY, server.spec.id.as_str()));
        let mut selected: u64 = ui
            .ctx()
            .data(|d| d.get_temp(selected_id))
            .filter(|id| runs.iter().any(|r| r.id == *id))
            .unwrap_or(runs[0].id);
        for run in &runs {
            ui.horizontal(|ui| {
                if ui
                    .selectable_label(run.id == selected, format::date_time(run.started_at))
                    .clicked()
                {
                    selected = run.id;
                }
                ui.label(RichText::new(&run.pipeline_name).strong());
                run_badge(ui, &run.status, &p);
                ui.label(RichText::new(run.summary()).color(p.text_secondary));
                if let Some(duration) = duration_of(run) {
                    ui.label(RichText::new(duration).color(p.text_muted));
                }
            });
        }
        ui.ctx().data_mut(|d| d.insert_temp(selected_id, selected));
        if let Some(run) = runs.iter().find(|r| r.id == selected) {
            ui.add_space(GAP);
            if let Some(next) = run_detail(ui, ctx, run) {
                action = Some(next);
            }
        }
    });
    action
}

fn run_detail(ui: &mut Ui, ctx: &DetailContext<'_>, run: &PipelineRun) -> Option<Action> {
    let p = Palette::current(ui.ctx());
    let mut action = None;
    ui.horizontal(|ui| {
        if run.is_running() {
            ui.spinner();
            if ui.button(text::BTN_CANCEL_RUN).clicked() {
                action = Some(Action::CancelPipeline(run.id));
            }
        } else if ui.button(text::BTN_RUN_AGAIN).clicked() {
            action = Some(Action::AskRunPipeline {
                server: ctx.server.spec.id.clone(),
                pipeline_id: run.pipeline.clone(),
            });
        }
    });
    let step_id = Id::new((SELECTED_STEP_KEY, run.id));
    let default_step = run
        .current_step()
        .or_else(|| {
            run.steps
                .iter()
                .rposition(|s| s.status != StepStatus::Pending)
        })
        .unwrap_or(0);
    let mut selected: usize = ui
        .ctx()
        .data(|d| d.get_temp(step_id))
        .unwrap_or(default_step)
        .min(run.steps.len().saturating_sub(1));
    let mut manual = false;
    ui.horizontal_wrapped(|ui| {
        for (index, step) in run.steps.iter().enumerate() {
            let label = format!("{}. {}", index + 1, step.name);
            if ui.selectable_label(index == selected, label).clicked() {
                selected = index;
                manual = true;
            }
            step_badge(ui, step, &p);
        }
    });
    if manual {
        ui.ctx().data_mut(|d| d.insert_temp(step_id, selected));
    } else if run.is_running() {
        ui.ctx().data_mut(|d| d.remove::<usize>(step_id));
    }
    if let Some(step) = run.steps.get(selected) {
        log_block(ui, step, run.is_running(), &p);
    }
    action
}

fn log_block(ui: &mut Ui, step: &StepRun, is_live: bool, p: &Palette) {
    let frame = egui::Frame::new()
        .fill(p.bg_panel)
        .inner_margin(GAP)
        .corner_radius(4.0);
    frame.show(ui, |ui| {
        ui.set_width(ui.available_width());
        scroll::vertical()
            .id_salt(("pipeline-log", step.name.as_str()))
            .max_height(LOG_HEIGHT)
            .stick_to_bottom(is_live)
            .show(ui, |ui| {
                if step.output.is_empty() {
                    ui.label(RichText::new(text::RUN_LOG_EMPTY).color(p.text_muted));
                } else {
                    ui.add(
                        egui::Label::new(RichText::new(&step.output).monospace())
                            .selectable(true)
                            .wrap(),
                    );
                }
            });
    });
}

fn run_badge(ui: &mut Ui, status: &RunStatus, p: &Palette) {
    match status {
        RunStatus::Running => badge(ui, text::RUN_STATUS_RUNNING, p.accent),
        RunStatus::Done => badge(ui, text::RUN_STATUS_DONE, p.ok),
        RunStatus::Failed(_) => badge(ui, text::RUN_STATUS_FAILED, p.critical),
        RunStatus::Cancelled => badge(ui, text::RUN_STATUS_CANCELLED, p.warning),
    }
}

fn step_badge(ui: &mut Ui, step: &StepRun, p: &Palette) {
    match step.status {
        StepStatus::Pending => badge(ui, text::STEP_STATUS_PENDING, p.text_muted),
        StepStatus::Running => badge(ui, text::STEP_STATUS_RUNNING, p.accent),
        StepStatus::Done => badge(ui, text::STEP_STATUS_DONE, p.ok),
        StepStatus::Failed => {
            let label = match step.exit_code {
                Some(code) => format!("{} ({code})", text::STEP_STATUS_FAILED),
                None => text::STEP_STATUS_FAILED.to_owned(),
            };
            badge(ui, &label, p.critical)
        }
        StepStatus::Skipped => badge(ui, text::STEP_STATUS_SKIPPED, p.text_muted),
    }
}

fn duration_of(run: &PipelineRun) -> Option<String> {
    let end = run.finished_at.unwrap_or_else(chrono::Utc::now);
    let seconds = (end - run.started_at).num_seconds().max(0) as f64;
    Some(format::duration_short(seconds))
}
