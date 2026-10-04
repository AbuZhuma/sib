mod draft;
mod editor;

use std::path::{Path, PathBuf};

use egui::{Align, Id, Layout, RichText, TextEdit, Ui};
use sib_core::Pipeline;

use self::draft::Draft;
use super::Action;
use crate::components::{help, page_title, panel, panel_with_controls, scroll};
use crate::text;
use crate::theme::{GAP, Palette};

const DRAFT_KEY: &str = "pipeline-draft";
const IMPORT_KEY: &str = "pipeline-import-path";
const DELETE_KEY: &str = "pipeline-delete-armed";
const PATH_FIELD: f32 = 360.0;

pub struct PipelinesContext<'a> {
    pub prototypes: &'a [Pipeline],
    pub export_dir: &'a Path,
}

pub fn show(ui: &mut Ui, ctx: &PipelinesContext<'_>) -> Option<Action> {
    page_title(ui, text::PIPELINES_TITLE);
    let mut draft: Option<Draft> = ui.ctx().data(|d| d.get_temp(Id::new(DRAFT_KEY)));
    let mut action = None;
    scroll::vertical().show(ui, |ui| {
        action = list(ui, ctx, &mut draft);
        if let Some(current) = &mut draft {
            ui.add_space(GAP);
            let (next, close) = editor_panel(ui, current);
            if next.is_some() {
                action = next;
            }
            if close {
                draft = None;
            }
        }
    });
    let is_saved = matches!(action, Some(Action::SavePipeline(_)));
    ui.ctx().data_mut(|d| {
        if is_saved || draft.is_none() {
            d.remove::<Draft>(Id::new(DRAFT_KEY));
        } else if let Some(draft) = draft {
            d.insert_temp(Id::new(DRAFT_KEY), draft);
        }
    });
    action
}

fn list(ui: &mut Ui, ctx: &PipelinesContext<'_>, draft: &mut Option<Draft>) -> Option<Action> {
    let p = Palette::current(ui.ctx());
    let mut action = None;
    let mut new_clicked = false;
    panel_with_controls(
        ui,
        text::PIPELINES_LIST,
        |ui| {
            help(ui, text::PIPELINES_HINT);
            new_clicked = ui.button(text::BTN_NEW_PIPELINE).clicked();
        },
        |ui| {
            if ctx.prototypes.is_empty() {
                ui.label(RichText::new(text::PIPELINES_EMPTY).color(p.text_muted));
            }
            for pipeline in ctx.prototypes {
                if let Some(next) = row(ui, ctx, pipeline, draft) {
                    action = Some(next);
                }
            }
            ui.add_space(GAP);
            if let Some(next) = import_row(ui) {
                action = Some(next);
            }
        },
    );
    if new_clicked {
        *draft = Some(Draft::new());
    }
    action
}

fn row(
    ui: &mut Ui,
    ctx: &PipelinesContext<'_>,
    pipeline: &Pipeline,
    draft: &mut Option<Draft>,
) -> Option<Action> {
    let p = Palette::current(ui.ctx());
    let mut action = None;
    let delete_id = Id::new((DELETE_KEY, pipeline.id.as_str()));
    let mut armed: bool = ui.ctx().data(|d| d.get_temp(delete_id)).unwrap_or(false);
    ui.horizontal(|ui| {
        ui.label(RichText::new(&pipeline.name).strong());
        ui.label(
            RichText::new(&pipeline.id)
                .monospace()
                .color(p.text_secondary),
        );
        ui.label(
            RichText::new(format!(
                "{} {}",
                pipeline.steps.len(),
                text::RUN_DIALOG_STEPS
            ))
            .color(p.text_secondary),
        );
        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
            if armed {
                if ui
                    .button(RichText::new(text::CONFIRM_DELETE_PIPELINE).color(p.critical))
                    .clicked()
                {
                    action = Some(Action::DeletePipeline(pipeline.id.clone()));
                    armed = false;
                }
                if ui.small_button(text::BTN_CANCEL).clicked() {
                    armed = false;
                }
            } else if ui.small_button(text::BTN_DELETE).clicked() {
                armed = true;
            }
            if ui.small_button(text::BTN_EXPORT).clicked() {
                action = Some(Action::ExportPipeline {
                    id: pipeline.id.clone(),
                    path: ctx.export_dir.join(format!("{}.toml", pipeline.id)),
                });
            }
            if ui.small_button(text::BTN_DUPLICATE).clicked() {
                *draft = Some(Draft::duplicate(pipeline));
            }
            if ui.small_button(text::BTN_EDIT).clicked() {
                *draft = Some(Draft::edit(pipeline));
            }
        });
    });
    if !pipeline.description.is_empty() {
        ui.add(
            egui::Label::new(RichText::new(&pipeline.description).color(p.text_muted)).truncate(),
        );
    }
    ui.add_space(GAP);
    ui.ctx().data_mut(|d| d.insert_temp(delete_id, armed));
    action
}

fn import_row(ui: &mut Ui) -> Option<Action> {
    let id = Id::new(IMPORT_KEY);
    let mut path: String = ui.ctx().data(|d| d.get_temp(id)).unwrap_or_default();
    let mut action = None;
    ui.horizontal(|ui| {
        ui.add(
            TextEdit::singleline(&mut path)
                .desired_width(PATH_FIELD)
                .hint_text(text::IMPORT_PATH),
        );
        if ui
            .add_enabled(!path.trim().is_empty(), egui::Button::new(text::BTN_IMPORT))
            .clicked()
        {
            action = Some(Action::ImportPipeline(PathBuf::from(path.trim())));
            path.clear();
        }
    });
    ui.ctx().data_mut(|d| d.insert_temp(id, path));
    action
}

fn editor_panel(ui: &mut Ui, draft: &mut Draft) -> (Option<Action>, bool) {
    let mut action = None;
    let mut close = false;
    panel(ui, text::PIPELINE_EDITOR, |ui| {
        editor::show(ui, draft);
        ui.add_space(GAP);
        ui.horizontal(|ui| {
            if ui.button(text::BTN_SAVE).clicked() {
                match draft.pipeline.validate() {
                    Ok(()) => action = Some(Action::SavePipeline(draft.pipeline.clone())),
                    Err(error) => draft.error = Some(error.to_string()),
                }
            }
            if ui.button(text::BTN_CANCEL).clicked() {
                close = true;
            }
            if draft.is_rename() {
                let p = Palette::current(ui.ctx());
                ui.label(
                    RichText::new(format!(
                        "{} → {}",
                        draft.original_id.clone().unwrap_or_default(),
                        draft.pipeline.id
                    ))
                    .color(p.warning),
                );
            }
        });
    });
    (action, close)
}
