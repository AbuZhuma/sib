use asiba_core::{AppState, AuditReport, AuditScope, AuditTarget};
use egui::{RichText, Ui};

use super::{Action, Page};
use crate::components::{AiBlock, ai_block, badge, first_section, panel, status_badge};
use crate::format;
use crate::pages::server_detail::audit::scope_label;
use crate::text;
use crate::theme::{GAP, Palette};

const MAX_INCIDENT_REPORTS: usize = 5;

pub fn show(ui: &mut Ui, state: &AppState, consent: bool, can_audit: bool) -> Option<Action> {
    if !consent {
        return None;
    }
    let block = AiBlock {
        state,
        target: AuditTarget::Fleet,
        scope: AuditScope::Full,
        title: text::AI_BLOCK_FLEET,
        can_audit,
        auto_request: false,
    };
    let mut action = ai_block(ui, &block);
    ui.add_space(GAP);
    if let Some(next) = queue(ui, state) {
        action = Some(next);
    }
    if let Some(next) = incident_reports(ui, state) {
        action = Some(next);
    }
    action
}

fn queue(ui: &mut Ui, state: &AppState) -> Option<Action> {
    let p = Palette::current(ui.ctx());
    let pending: Vec<&AuditReport> = state.audits.iter().filter(|a| a.is_running()).collect();
    if pending.is_empty() {
        return None;
    }
    let mut action = None;
    let title = format!("{} ({})", text::AI_QUEUE, pending.len());
    panel(ui, &title, |ui| {
        for report in pending {
            ui.horizontal_wrapped(|ui| {
                status_badge(ui, &report.status, &p);
                match report.target.server() {
                    Some(server) if ui.link(server.as_str()).clicked() => {
                        action = Some(Action::Navigate(Page::ServerDetail(server.clone())));
                    }
                    Some(_) => {}
                    None => {
                        ui.label(text::AI_TARGET_FLEET);
                    }
                }
                ui.label(scope_label(&report.scope));
                ui.label(
                    RichText::new(format::clock(report.started_at))
                        .small()
                        .color(p.text_muted),
                );
            });
        }
    });
    ui.add_space(GAP);
    action
}

fn incident_reports(ui: &mut Ui, state: &AppState) -> Option<Action> {
    let p = Palette::current(ui.ctx());
    let reports: Vec<&AuditReport> = state
        .audits
        .iter()
        .rev()
        .filter(|a| a.scope.is_incident() && a.is_done())
        .take(MAX_INCIDENT_REPORTS)
        .collect();
    if reports.is_empty() {
        return None;
    }
    let mut action = None;
    panel(ui, text::AI_INCIDENT_REPORTS, |ui| {
        for report in reports {
            let Some(server) = report.target.server() else {
                continue;
            };
            let subject = match &report.scope {
                AuditScope::Incident { subject, .. } => subject.as_str(),
                _ => "",
            };
            ui.horizontal_wrapped(|ui| {
                if ui.link(server.as_str()).clicked() {
                    action = Some(Action::Navigate(Page::ServerDetail(server.clone())));
                }
                badge(ui, subject, p.warning);
                ui.label(
                    RichText::new(format::clock(report.started_at))
                        .small()
                        .color(p.text_muted),
                );
                ui.label(RichText::new(first_section(&report.text)).color(p.text_secondary));
            });
        }
    });
    ui.add_space(GAP);
    action
}
