use asiba_config::{AiConfig, AiProvider, AppConfig};
use asiba_core::{AppState, Severity};
use egui::{ComboBox, Grid, Id, RichText, TextEdit, Ui};

use super::Action;
use crate::components::chip_value;
use crate::modules::Tab;
use crate::text;
use crate::theme::{GAP, Palette};

const DRAFT_KEY: &str = "ai-settings-draft";
const KEY_FIELD: f32 = 420.0;
const MODEL_FIELD: f32 = 200.0;
const NUMBER_FIELD: f32 = 80.0;
const SECONDS_PER_MINUTE: u64 = 60;

#[derive(Clone, Debug, PartialEq, Eq)]
struct Draft {
    consent: bool,
    provider: AiProvider,
    api_key: String,
    model: String,
    base_url: String,
    context_tokens: String,
    auto_audit: bool,
    min_severity: Severity,
    cooldown_minutes: String,
    auto_sections: bool,
    sections: Vec<String>,
}

impl Draft {
    fn from_config(config: &AiConfig) -> Self {
        Self {
            consent: config.consent,
            provider: config.provider,
            api_key: config.api_key.clone(),
            model: config.model.clone(),
            base_url: config.base_url.clone(),
            context_tokens: config.context_tokens.to_string(),
            auto_audit: config.auto_audit,
            min_severity: config.auto_audit_min_severity,
            cooldown_minutes: (config.cooldown_secs / SECONDS_PER_MINUTE).to_string(),
            auto_sections: config.auto_sections,
            sections: config.sections.clone(),
        }
    }

    fn toggle_section(&mut self, key: &str, is_enabled: bool) {
        self.sections.retain(|s| s != key);
        if is_enabled {
            self.sections.push(key.to_owned());
        }
    }

    fn build(&self) -> Option<AiConfig> {
        let model = self.model.trim();
        if model.is_empty() {
            return None;
        }
        Some(AiConfig {
            consent: self.consent,
            provider: self.provider,
            api_key: self.api_key.trim().to_owned(),
            model: model.to_owned(),
            base_url: self.base_url.trim().to_owned(),
            context_tokens: self.context_tokens.trim().parse().ok()?,
            auto_audit: self.auto_audit,
            auto_audit_min_severity: self.min_severity,
            cooldown_secs: self.cooldown_minutes.trim().parse::<u64>().ok()? * SECONDS_PER_MINUTE,
            auto_sections: self.auto_sections,
            sections: self.sections.clone(),
        })
    }
}

pub fn show(ui: &mut Ui, config: &AppConfig, state: &AppState) -> Option<Action> {
    let p = Palette::current(ui.ctx());
    let id = Id::new(DRAFT_KEY);
    let mut draft: Draft = ui
        .ctx()
        .data(|d| d.get_temp(id))
        .unwrap_or_else(|| Draft::from_config(&config.ai));
    status_line(ui, &config.ai, state);
    ui.add_space(GAP);
    ui.label(RichText::new(text::AI_PRIVACY).color(p.text_secondary));
    ui.add_space(GAP);
    ui.checkbox(&mut draft.consent, text::AI_CONSENT);
    ui.add_space(GAP);
    provider_picker(ui, &mut draft, &p);
    Grid::new("ai-grid")
        .num_columns(2)
        .spacing([12.0, 4.0])
        .show(ui, |ui| fields(ui, &mut draft));
    ui.add_space(GAP);
    auto_audit_fields(ui, &mut draft, &p);
    ui.add_space(GAP);
    section_picker(ui, &mut draft, &p);
    ui.label(RichText::new(text::AI_HINT).small().color(p.text_muted));
    ui.add_space(GAP);
    let action = apply_button(ui, &draft, config, state);
    ui.ctx().data_mut(|d| d.insert_temp(id, draft));
    if action.is_some() {
        ui.ctx().data_mut(|d| d.remove::<Draft>(id));
    }
    action
}

fn auto_audit_fields(ui: &mut Ui, draft: &mut Draft, p: &Palette) {
    ui.checkbox(&mut draft.auto_audit, text::AI_AUTO_AUDIT);
    ui.horizontal(|ui| {
        ui.label(RichText::new(text::AI_MIN_SEVERITY).color(p.text_secondary));
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
}

fn section_picker(ui: &mut Ui, draft: &mut Draft, p: &Palette) {
    ui.checkbox(&mut draft.auto_sections, text::AI_AUTO_SECTIONS);
    ui.label(
        RichText::new(text::AI_SECTIONS_HINT)
            .small()
            .color(p.text_muted),
    );
    let mut seen: Vec<&str> = Vec::new();
    ui.horizontal_wrapped(|ui| {
        for tab in Tab::ALL {
            let Some(key) = tab.section_key() else {
                continue;
            };
            if seen.contains(&key) {
                continue;
            }
            seen.push(key);
            let mut is_enabled = draft.sections.iter().any(|s| s == key);
            let changed = ui
                .add_enabled(
                    draft.auto_sections,
                    egui::Checkbox::new(&mut is_enabled, tab.label()),
                )
                .changed();
            if changed {
                draft.toggle_section(key, is_enabled);
            }
        }
    });
}

fn model_selector(ui: &mut Ui, draft: &mut Draft) {
    ComboBox::from_id_salt("ai-model")
        .width(MODEL_FIELD)
        .selected_text(draft.model.clone())
        .show_ui(ui, |ui| {
            for model in draft.provider.models() {
                ui.selectable_value(&mut draft.model, (*model).to_owned(), *model);
            }
        });
}

fn provider_picker(ui: &mut Ui, draft: &mut Draft, p: &Palette) {
    ui.horizontal_wrapped(|ui| {
        ui.label(RichText::new(text::AI_PROVIDER).color(p.text_secondary));
        let previous = draft.provider;
        for provider in AiProvider::ALL {
            chip_value(ui, &mut draft.provider, provider, provider.label());
        }
        let known = draft.provider.models().contains(&draft.model.trim());
        if draft.provider != previous && (!known || !draft.provider.needs_base_url()) {
            let keep = previous.needs_base_url() && !draft.model.trim().is_empty();
            if !known || !keep {
                draft.model = draft.provider.default_model().to_owned();
            }
        }
    });
}

fn fields(ui: &mut Ui, draft: &mut Draft) {
    let p = Palette::current(ui.ctx());
    ui.label(RichText::new(text::AI_API_KEY).color(p.text_secondary));
    ui.add(
        TextEdit::singleline(&mut draft.api_key)
            .password(true)
            .desired_width(KEY_FIELD),
    );
    ui.end_row();
    if draft.provider.needs_base_url() {
        text_row(ui, text::AI_BASE_URL, &mut draft.base_url, KEY_FIELD);
        text_row(ui, text::AI_MODEL, &mut draft.model, MODEL_FIELD);
    } else {
        ui.label(RichText::new(text::AI_MODEL).color(p.text_secondary));
        model_selector(ui, draft);
        ui.end_row();
    }
    text_row(
        ui,
        text::AI_CONTEXT,
        &mut draft.context_tokens,
        NUMBER_FIELD,
    );
    text_row(
        ui,
        text::AI_COOLDOWN,
        &mut draft.cooldown_minutes,
        NUMBER_FIELD,
    );
}

fn text_row(ui: &mut Ui, label: &str, value: &mut String, width: f32) {
    let p = Palette::current(ui.ctx());
    ui.label(RichText::new(label).color(p.text_secondary));
    ui.add(TextEdit::singleline(value).desired_width(width));
    ui.end_row();
}

fn status_line(ui: &mut Ui, config: &AiConfig, state: &AppState) {
    let p = Palette::current(ui.ctx());
    let running = state.audits.iter().filter(|a| a.is_running()).count();
    let (label, color) = if !config.consent {
        (text::AI_STATUS_OFF, p.text_muted)
    } else if !config.is_ready() {
        (text::AI_STATUS_NO_KEY, p.warning)
    } else if running > 0 {
        (text::AI_STATUS_BUSY, p.accent)
    } else {
        (text::AI_STATUS_READY, p.ok)
    };
    ui.label(RichText::new(label).color(color));
}

fn apply_button(
    ui: &mut Ui,
    draft: &Draft,
    config: &AppConfig,
    state: &AppState,
) -> Option<Action> {
    let parsed = draft.build();
    let is_dirty = *draft != Draft::from_config(&config.ai);
    let mut action = None;
    ui.horizontal(|ui| {
        if ui
            .add_enabled(
                parsed.is_some() && is_dirty,
                egui::Button::new(text::BTN_APPLY),
            )
            .clicked()
        {
            action = parsed.clone().map(Action::SaveAiConfig);
        }
        let done = state.audits.iter().filter(|a| !a.is_running()).count();
        ui.label(
            RichText::new(format!("{} {done}", text::AI_REPORTS_COUNT))
                .small()
                .color(Palette::current(ui.ctx()).text_muted),
        );
    });
    action
}
