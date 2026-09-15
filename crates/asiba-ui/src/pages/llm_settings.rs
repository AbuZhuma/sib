use std::path::PathBuf;

use asiba_config::{AppConfig, LlmConfig};
use asiba_core::{AppState, Severity};
use asiba_llm::{InstallProgress, InstallStep};
use egui::{Grid, Id, ProgressBar, RichText, TextEdit, Ui};

use super::Action;
use crate::components::chip_value;
use crate::format;
use crate::text;
use crate::theme::{GAP, Palette};

const DRAFT_KEY: &str = "llm-settings-draft";
const PATH_FIELD: f32 = 420.0;
const NUMBER_FIELD: f32 = 80.0;
const SECONDS_PER_MINUTE: u64 = 60;
const PROGRESS_WIDTH: f32 = 420.0;

#[derive(Clone, Debug, PartialEq, Eq)]
struct Draft {
    enabled: bool,
    model_path: String,
    server_binary: String,
    threads: String,
    context_tokens: String,
    auto_audit: bool,
    min_severity: Severity,
    cooldown_minutes: String,
    idle_minutes: String,
}

impl Draft {
    fn from_config(config: &LlmConfig) -> Self {
        Self {
            enabled: config.enabled,
            model_path: config
                .model_path
                .as_ref()
                .map(|p| p.display().to_string())
                .unwrap_or_default(),
            server_binary: config.server_binary.clone(),
            threads: config.threads.to_string(),
            context_tokens: config.context_tokens.to_string(),
            auto_audit: config.auto_audit,
            min_severity: config.auto_audit_min_severity,
            cooldown_minutes: (config.cooldown_secs / SECONDS_PER_MINUTE).to_string(),
            idle_minutes: (config.idle_unload_secs / SECONDS_PER_MINUTE).to_string(),
        }
    }

    fn build(&self) -> Option<LlmConfig> {
        let model_path = self.model_path.trim();
        Some(LlmConfig {
            enabled: self.enabled,
            model_path: (!model_path.is_empty()).then(|| PathBuf::from(model_path)),
            server_binary: self.server_binary.trim().to_owned(),
            threads: self.threads.trim().parse().ok()?,
            context_tokens: self.context_tokens.trim().parse().ok()?,
            auto_audit: self.auto_audit,
            auto_audit_min_severity: self.min_severity,
            cooldown_secs: self.cooldown_minutes.trim().parse::<u64>().ok()? * SECONDS_PER_MINUTE,
            idle_unload_secs: self.idle_minutes.trim().parse::<u64>().ok()? * SECONDS_PER_MINUTE,
        })
    }
}

pub fn show(
    ui: &mut Ui,
    config: &AppConfig,
    state: &AppState,
    install: Option<InstallProgress>,
) -> Option<Action> {
    let p = Palette::current(ui.ctx());
    if let Some(progress) = install {
        installing(ui, progress);
        return None;
    }
    if !config.llm.is_ready() && install_block(ui, &p) {
        return Some(Action::InstallLlm);
    }
    let id = Id::new(DRAFT_KEY);
    let mut draft: Draft = ui
        .ctx()
        .data(|d| d.get_temp(id))
        .unwrap_or_else(|| Draft::from_config(&config.llm));
    status_line(ui, &config.llm, state);
    ui.add_space(GAP);
    ui.checkbox(&mut draft.enabled, text::LLM_ENABLED);
    Grid::new("llm-grid")
        .num_columns(2)
        .spacing([12.0, 4.0])
        .show(ui, |ui| fields(ui, &mut draft));
    ui.add_space(GAP);
    ui.checkbox(&mut draft.auto_audit, text::LLM_AUTO_AUDIT);
    ui.horizontal(|ui| {
        ui.label(RichText::new(text::LLM_MIN_SEVERITY).color(p.text_secondary));
        chip_value(
            ui,
            &mut draft.min_severity,
            Severity::Warning,
            text::SEVERITY_WARNING,
        );
        chip_value(
            ui,
            &mut draft.min_severity,
            Severity::Critical,
            text::SEVERITY_CRITICAL,
        );
    });
    ui.label(RichText::new(text::LLM_HINT).small().color(p.text_muted));
    ui.add_space(GAP);
    let action = buttons(ui, &draft, config, state);
    ui.ctx().data_mut(|d| d.insert_temp(id, draft));
    if action.is_some() {
        ui.ctx().data_mut(|d| d.remove::<Draft>(id));
    }
    action
}

fn install_block(ui: &mut Ui, p: &Palette) -> bool {
    let clicked = ui.button(text::LLM_INSTALL).clicked();
    ui.label(
        RichText::new(text::LLM_INSTALL_HINT)
            .small()
            .color(p.text_muted),
    );
    ui.add_space(GAP);
    clicked
}

fn installing(ui: &mut Ui, progress: InstallProgress) {
    let p = Palette::current(ui.ctx());
    let step = match progress.step {
        InstallStep::LlamaDownload => text::LLM_INSTALLING_LLAMA,
        InstallStep::LlamaExtract => text::LLM_INSTALLING_EXTRACT,
        InstallStep::ModelDownload => text::LLM_INSTALLING_MODEL,
        InstallStep::Done => text::LLM_INSTALLING_DONE,
    };
    let done = format::bytes(progress.done_bytes);
    let label = match progress.total_bytes {
        Some(total) => format!("{step}: {done} / {}", format::bytes(total)),
        None => format!("{step}: {done}"),
    };
    ui.label(RichText::new(label).color(p.text));
    let fraction = progress
        .total_bytes
        .filter(|t| *t > 0)
        .map(|t| progress.done_bytes as f32 / t as f32)
        .unwrap_or(0.0);
    ui.add(ProgressBar::new(fraction).desired_width(PROGRESS_WIDTH));
    ui.ctx().request_repaint();
}

fn fields(ui: &mut Ui, draft: &mut Draft) {
    let p = Palette::current(ui.ctx());
    let text_row = |ui: &mut Ui, label: &str, value: &mut String, width: f32| {
        ui.label(RichText::new(label).color(p.text_secondary));
        ui.add(TextEdit::singleline(value).desired_width(width));
        ui.end_row();
    };
    text_row(ui, text::LLM_MODEL_PATH, &mut draft.model_path, PATH_FIELD);
    text_row(
        ui,
        text::LLM_SERVER_BINARY,
        &mut draft.server_binary,
        PATH_FIELD,
    );
    text_row(ui, text::LLM_THREADS, &mut draft.threads, NUMBER_FIELD);
    text_row(
        ui,
        text::LLM_CONTEXT,
        &mut draft.context_tokens,
        NUMBER_FIELD,
    );
    text_row(
        ui,
        text::LLM_COOLDOWN,
        &mut draft.cooldown_minutes,
        NUMBER_FIELD,
    );
    text_row(ui, text::LLM_IDLE, &mut draft.idle_minutes, NUMBER_FIELD);
}

fn status_line(ui: &mut Ui, config: &LlmConfig, state: &AppState) {
    let p = Palette::current(ui.ctx());
    let running = state.audits.iter().filter(|a| a.is_running()).count();
    let (label, color) = if !config.enabled {
        (text::LLM_STATUS_OFF, p.text_muted)
    } else if !config.is_ready() {
        (text::LLM_STATUS_NO_MODEL, p.warning)
    } else if running > 0 {
        (text::LLM_STATUS_BUSY, p.accent)
    } else {
        (text::LLM_STATUS_READY, p.ok)
    };
    ui.label(RichText::new(label).color(color));
}

fn buttons(ui: &mut Ui, draft: &Draft, config: &AppConfig, state: &AppState) -> Option<Action> {
    let parsed = draft.build();
    let is_dirty = *draft != Draft::from_config(&config.llm);
    let mut action = None;
    ui.horizontal(|ui| {
        if ui
            .add_enabled(
                parsed.is_some() && is_dirty,
                egui::Button::new(text::BTN_APPLY),
            )
            .clicked()
        {
            action = parsed.clone().map(Action::SaveLlmConfig);
        }
        if ui.button(text::LLM_UNLOAD).clicked() {
            action = Some(Action::UnloadModel);
        }
        let done = state.audits.iter().filter(|a| !a.is_running()).count();
        ui.label(
            RichText::new(format!("{} {done}", text::LLM_REPORTS_COUNT))
                .small()
                .color(Palette::current(ui.ctx()).text_muted),
        );
    });
    action
}
