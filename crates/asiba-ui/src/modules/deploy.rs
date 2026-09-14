use asiba_core::{ModuleId, ServerState};
use asiba_modules::deploy::{self, Deploy, DeployState, DeployStatus, StageStatus};
use chrono::Utc;
use egui::{Color32, CornerRadius, Frame, Id, Margin, RichText, Stroke, TextEdit, Ui};

use super::{ModuleView, Tab, ViewAction, ViewShared};
use crate::components::{Table, badge};
use crate::format;
use crate::text;
use crate::theme::{GAP, Palette};

const SUMMARY_SHOWN: usize = 6;
const FILTER_KEY: &str = "deploy-filter";
const OPEN_KEY: &str = "deploy-open";

pub struct DeployView;

impl ModuleView for DeployView {
    fn id(&self) -> ModuleId {
        deploy::ID
    }

    fn title(&self) -> &'static str {
        text::MODULE_DEPLOY
    }

    fn tab(&self) -> Tab {
        Tab::Deploy
    }

    fn summary(&self, ui: &mut Ui, server: &ServerState) {
        let p = Palette::current(ui.ctx());
        let Some(state) = server.data::<DeployState>(deploy::ID) else {
            return;
        };
        let snapshot = &state.snapshot;
        if snapshot.deploys.is_empty() {
            ui.label(RichText::new(text::DEP_NONE).color(p.text_muted));
            return;
        }
        let active = snapshot.active().count();
        let failed = snapshot.failed_count();
        ui.horizontal_wrapped(|ui| {
            if active > 0 {
                ui.label(
                    RichText::new(format!("{active} {}", text::DEP_IN_PROGRESS))
                        .monospace()
                        .color(p.info),
                );
            }
            if failed > 0 {
                ui.label(
                    RichText::new(format!("{failed} {}", text::DEP_FAILED_COUNT))
                        .monospace()
                        .color(p.critical),
                );
            }
            runner_line(ui, state, &p);
        });
        for deploy in snapshot.deploys.iter().take(SUMMARY_SHOWN) {
            ui.horizontal(|ui| {
                status_badge(ui, deploy.status, &p);
                ui.label(RichText::new(&deploy.project).strong());
                ui.label(RichText::new(deploy.source.label()).color(p.text_muted));
                ui.label(
                    RichText::new(format::clock(deploy.started_at))
                        .monospace()
                        .color(p.text_secondary),
                );
                timeline(ui, deploy, &p);
            });
        }
    }

    fn page(&self, ui: &mut Ui, server: &ServerState, _shared: &ViewShared) -> Option<ViewAction> {
        let p = Palette::current(ui.ctx());
        let state = server.data::<DeployState>(deploy::ID)?;
        let snapshot = &state.snapshot;
        runner_line(ui, state, &p);
        let active: Vec<&Deploy> = snapshot.active().collect();
        if !active.is_empty() {
            section_title(ui, text::DEP_ACTIVE);
            for deploy in active {
                deploy_card(ui, deploy, &p, true);
            }
            ui.add_space(GAP);
        }
        section_title(ui, text::DEP_HISTORY);
        let filter_id = Id::new((FILTER_KEY, server.spec.id.as_str()));
        let mut filter: String = ui.ctx().data(|d| d.get_temp(filter_id)).unwrap_or_default();
        ui.add(
            TextEdit::singleline(&mut filter)
                .hint_text(text::DEP_FILTER)
                .desired_width(260.0),
        );
        ui.ctx()
            .data_mut(|d| d.insert_temp(filter_id, filter.clone()));
        let needle = filter.trim().to_lowercase();
        history_table(ui, snapshot, &needle, &p);
        None
    }
}

fn runner_line(ui: &mut Ui, state: &DeployState, p: &Palette) {
    let Some(runner) = &state.snapshot.runner else {
        return;
    };
    ui.horizontal_wrapped(|ui| {
        ui.label(RichText::new(text::DEP_RUNNER).color(p.text_secondary));
        ui.label(runner.kind.label());
        ui.monospace(&runner.working_dir);
        if runner.is_busy {
            badge(ui, text::DEP_RUNNER_BUSY, p.info);
        } else {
            badge(ui, text::DEP_RUNNER_IDLE, p.text_muted);
        }
    });
}

fn section_title(ui: &mut Ui, title: &str) {
    let p = Palette::current(ui.ctx());
    ui.label(
        RichText::new(title.to_uppercase())
            .small()
            .color(p.text_secondary),
    );
}

fn status_badge(ui: &mut Ui, status: DeployStatus, p: &Palette) {
    let (label, color) = status_style(status, p);
    badge(ui, label, color);
}

fn status_style(status: DeployStatus, p: &Palette) -> (&'static str, Color32) {
    match status {
        DeployStatus::InProgress => (text::DEP_IN_PROGRESS, p.info),
        DeployStatus::Success => (text::DEP_SUCCESS, p.ok),
        DeployStatus::Failed => (text::DEP_FAILED, p.critical),
    }
}

pub fn timeline(ui: &mut Ui, deploy: &Deploy, p: &Palette) {
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = 2.0;
        for stage in &deploy.stages {
            let (fill, stroke, color) = match stage.status {
                StageStatus::Done => (p.bg_raised, p.border, p.text_secondary),
                StageStatus::Active => (p.accent_bg, p.accent, p.text),
                StageStatus::Failed => (p.bg_raised, p.critical, p.critical),
            };
            Frame::new()
                .fill(fill)
                .stroke(Stroke::new(1.0, stroke))
                .corner_radius(CornerRadius::same(2))
                .inner_margin(Margin::symmetric(6, 1))
                .show(ui, |ui| {
                    let label = match stage.at {
                        Some(at) => format!("{} {}", stage.name, format::clock(at)),
                        None => stage.name.clone(),
                    };
                    ui.label(RichText::new(label).small().color(color));
                });
        }
    });
}

fn deploy_card(ui: &mut Ui, deploy: &Deploy, p: &Palette, show_log: bool) {
    let now = Utc::now();
    ui.horizontal(|ui| {
        status_badge(ui, deploy.status, p);
        ui.label(RichText::new(&deploy.project).strong());
        ui.label(RichText::new(deploy.source.label()).color(p.text_muted));
        ui.monospace(format::date_time(deploy.started_at));
        ui.monospace(format::duration_short(deploy.duration_secs(now) as f64));
    });
    ui.label(RichText::new(&deploy.detail).color(p.text_secondary));
    timeline(ui, deploy, p);
    if let Some(error) = &deploy.error {
        error_block(ui, error, p);
    }
    if show_log && !deploy.log_tail.is_empty() {
        log_block(ui, &deploy.log_tail, p);
    }
    ui.add_space(GAP);
}

fn error_block(ui: &mut Ui, error: &deploy::DeployError, p: &Palette) {
    Frame::new()
        .stroke(Stroke::new(1.0, p.critical))
        .corner_radius(CornerRadius::same(2))
        .inner_margin(Margin::same(6))
        .show(ui, |ui| {
            ui.label(RichText::new(text::DEP_ERROR_AT).small().color(p.critical));
            ui.monospace(RichText::new(&error.line).color(p.critical));
            if !error.context.is_empty() {
                ui.collapsing(text::DEP_CONTEXT, |ui| {
                    ui.monospace(error.context.join("\n"));
                });
            }
        });
}

fn log_block(ui: &mut Ui, lines: &[String], p: &Palette) {
    ui.collapsing(text::DEP_LOG, |ui| {
        ui.monospace(RichText::new(lines.join("\n")).color(p.text_secondary));
    });
}

fn history_table(ui: &mut Ui, snapshot: &deploy::DeploySnapshot, needle: &str, p: &Palette) {
    let now = Utc::now();
    let open_id = Id::new(OPEN_KEY);
    let mut open: Option<String> = ui.ctx().data(|d| d.get_temp(open_id));
    let columns = [
        text::COL_STATUS,
        text::DEP_PROJECT,
        text::DEP_SOURCE,
        text::DEP_STARTED,
        text::DEP_DURATION,
        text::DEP_STAGE,
        "",
    ];
    Table::new("deploy-history", &columns).show(ui, |ui| {
        for deploy in snapshot
            .deploys
            .iter()
            .filter(|d| matches_filter(d, needle))
        {
            status_badge(ui, deploy.status, p);
            ui.label(&deploy.project);
            ui.label(RichText::new(deploy.source.label()).color(p.text_secondary));
            ui.monospace(format::date_time(deploy.started_at));
            ui.monospace(format::duration_short(deploy.duration_secs(now) as f64));
            timeline(ui, deploy, p);
            let is_open = open.as_deref() == Some(&deploy.key);
            let label = if is_open {
                text::DEP_HIDE
            } else {
                text::DEP_SHOW
            };
            if ui.small_button(label).clicked() {
                open = if is_open {
                    None
                } else {
                    Some(deploy.key.clone())
                };
            }
            ui.end_row();
        }
    });
    ui.ctx().data_mut(|d| d.insert_temp(open_id, open.clone()));
    if let Some(deploy) = open.and_then(|key| snapshot.find(&key)) {
        ui.add_space(GAP);
        deploy_card(ui, deploy, p, true);
    }
}

fn matches_filter(deploy: &Deploy, needle: &str) -> bool {
    needle.is_empty()
        || deploy.project.to_lowercase().contains(needle)
        || deploy.source.label().contains(needle)
        || deploy.detail.to_lowercase().contains(needle)
}
