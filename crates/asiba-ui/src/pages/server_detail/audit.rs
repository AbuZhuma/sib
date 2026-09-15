use asiba_core::{AuditReport, AuditScope, AuditStatus};
use egui::{Id, Label, RichText, Ui};

use super::DetailContext;
use crate::components::{badge, chip, panel};
use crate::format;
use crate::pages::Action;
use crate::text;
use crate::theme::{GAP, GAP_SMALL, Palette};

const SELECTED_KEY: &str = "audit-selected";

pub fn show(ui: &mut Ui, ctx: &DetailContext<'_>) -> Option<Action> {
    let p = Palette::current(ui.ctx());
    let reports: Vec<&AuditReport> = ctx
        .state
        .audits
        .iter()
        .rev()
        .filter(|a| a.server == ctx.server.spec.id)
        .collect();
    let running = reports.iter().any(|r| r.is_running());
    let mut action = None;
    ui.horizontal(|ui| {
        let ready = ctx.ai.is_ready() && !running;
        if ui
            .add_enabled(ready, egui::Button::new(text::AUDIT_FULL))
            .clicked()
        {
            action = Some(Action::Audit {
                server: ctx.server.spec.id.clone(),
                scope: AuditScope::Full,
            });
        }
        if !ctx.ai.is_ready() {
            ui.label(RichText::new(text::AUDIT_DISABLED).color(p.text_muted));
        } else if running {
            ui.label(RichText::new(text::AI_STATUS_BUSY).color(p.accent));
        }
    });
    ui.add_space(GAP);
    if reports.is_empty() {
        ui.label(RichText::new(text::AUDIT_EMPTY).color(p.text_muted));
        return action;
    }
    let selected = pick(ui, &reports);
    if let Some(report) = reports.iter().find(|r| r.id == selected) {
        report_panel(ui, report);
    }
    action
}

fn pick(ui: &mut Ui, reports: &[&AuditReport]) -> u64 {
    let id = Id::new(SELECTED_KEY);
    let stored: Option<u64> = ui.ctx().data(|d| d.get_temp(id));
    let mut selected = stored
        .filter(|s| reports.iter().any(|r| r.id == *s))
        .unwrap_or(reports[0].id);
    ui.horizontal_wrapped(|ui| {
        for report in reports {
            let label = format!(
                "{} · {}",
                format::clock(report.started_at),
                scope_label(&report.scope)
            );
            if chip(ui, report.id == selected, label).clicked() {
                selected = report.id;
            }
        }
    });
    ui.ctx().data_mut(|d| d.insert_temp(id, selected));
    ui.add_space(GAP);
    selected
}

fn scope_label(scope: &AuditScope) -> String {
    match scope {
        AuditScope::Full => text::AUDIT_FULL.to_lowercase(),
        AuditScope::Incident { subject, .. } => subject.clone(),
    }
}

fn report_panel(ui: &mut Ui, report: &AuditReport) {
    let p = Palette::current(ui.ctx());
    panel(ui, &scope_label(&report.scope), |ui| {
        ui.horizontal_wrapped(|ui| {
            match &report.status {
                AuditStatus::Running => badge(ui, text::AUDIT_RUNNING, p.accent),
                AuditStatus::Done => badge(ui, text::AUDIT_DONE, p.ok),
                AuditStatus::Failed(_) => badge(ui, text::AUDIT_FAILED, p.critical),
            }
            ui.label(RichText::new(&report.model).color(p.text_secondary));
            ui.label(
                RichText::new(format!("{} {}", report.context_tokens, text::AUDIT_TOKENS))
                    .small()
                    .color(p.text_muted),
            );
            if let Some(finished) = report.finished_at {
                let secs = (finished - report.started_at).num_seconds().max(0) as f64;
                ui.label(
                    RichText::new(format::duration_short(secs))
                        .small()
                        .color(p.text_muted),
                );
            }
        });
        ui.add_space(GAP_SMALL);
        match &report.status {
            AuditStatus::Failed(error) => {
                ui.label(RichText::new(error).color(p.critical));
            }
            AuditStatus::Running => {
                ui.spinner();
            }
            AuditStatus::Done => markdown_lite(ui, &report.text),
        }
    });
}

fn markdown_lite(ui: &mut Ui, text: &str) {
    let p = Palette::current(ui.ctx());
    let mut in_code = false;
    for line in text.lines() {
        if line.trim_start().starts_with("```") {
            in_code = !in_code;
            continue;
        }
        if in_code {
            ui.add(Label::new(RichText::new(line).monospace()).wrap());
            continue;
        }
        let trimmed = line.trim_start();
        if let Some(heading) = trimmed.strip_prefix('#') {
            ui.add_space(GAP_SMALL);
            ui.label(RichText::new(heading.trim_start_matches('#').trim()).strong());
            continue;
        }
        if let Some(item) = trimmed
            .strip_prefix("- ")
            .or_else(|| trimmed.strip_prefix("* "))
        {
            ui.add(Label::new(format!("• {}", strip_marks(item))).wrap());
            continue;
        }
        if trimmed.is_empty() {
            ui.add_space(GAP_SMALL);
            continue;
        }
        ui.add(Label::new(RichText::new(strip_marks(trimmed)).color(p.text)).wrap());
    }
}

fn strip_marks(text: &str) -> String {
    text.replace("**", "").replace('`', "")
}
