use asiba_core::{AuditReport, AuditScope, AuditTarget};
use egui::{Id, RichText, Ui};

use super::DetailContext;
use crate::components::{audit_status_label, chip, panel, report_body};
use crate::format;
use crate::pages::Action;
use crate::text;
use crate::theme::{GAP, Palette};

const SELECTED_KEY: &str = "audit-selected";

pub fn show(ui: &mut Ui, ctx: &DetailContext<'_>) -> Option<Action> {
    let p = Palette::current(ui.ctx());
    let reports: Vec<&AuditReport> = ctx
        .state
        .audits
        .iter()
        .rev()
        .filter(|a| a.is_for(&ctx.server.spec.id))
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
                target: AuditTarget::Server(ctx.server.spec.id.clone()),
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
        panel(ui, &scope_label(&report.scope), |ui| {
            report_body(ui, report, &p)
        });
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
                "{} · {} · {}",
                format::clock(report.started_at),
                scope_label(&report.scope),
                audit_status_label(&report.status)
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

pub fn scope_label(scope: &AuditScope) -> String {
    match scope {
        AuditScope::Full => text::AUDIT_FULL.to_lowercase(),
        AuditScope::Incident { subject, .. } => subject.clone(),
        AuditScope::Section { key } => format!("{} {key}", text::AUDIT_SECTION),
    }
}
