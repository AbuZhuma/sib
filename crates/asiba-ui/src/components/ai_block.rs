use asiba_core::{AppState, AuditReport, AuditScope, AuditStatus, AuditTarget};
use egui::{Id, Label, RichText, Ui};

use super::{badge, panel_with_controls};
use crate::format;
use crate::pages::Action;
use crate::text;
use crate::theme::{GAP_SMALL, Palette};

pub struct AiBlock<'a> {
    pub state: &'a AppState,
    pub target: AuditTarget,
    pub scope: AuditScope,
    pub title: &'a str,
    pub can_audit: bool,
    pub auto_request: bool,
}

pub fn ai_block(ui: &mut Ui, block: &AiBlock<'_>) -> Option<Action> {
    let p = Palette::current(ui.ctx());
    let report = block.state.latest_audit(&block.target, &block.scope);
    let running = report.is_some_and(AuditReport::is_running);
    let mut action = None;
    panel_with_controls(
        ui,
        block.title,
        |ui| {
            let label = if report.is_some() {
                text::AI_REFRESH
            } else {
                text::AI_ANALYZE
            };
            if let Some(cancel) = report.and_then(|r| cancel_button(ui, r)) {
                action = Some(cancel);
            } else if ui
                .add_enabled(
                    block.can_audit && !running,
                    egui::Button::new(label).small(),
                )
                .clicked()
            {
                action = Some(audit_action(block));
            }
        },
        |ui| match report {
            Some(report) => report_body(ui, report, &p),
            None => empty_body(ui, block, &p),
        },
    );
    if action.is_none() && report.is_none() && block.can_audit && block.auto_request {
        action = auto_once(ui, block);
    }
    action
}

fn audit_action(block: &AiBlock<'_>) -> Action {
    Action::Audit {
        target: block.target.clone(),
        scope: block.scope.clone(),
    }
}

fn auto_once(ui: &Ui, block: &AiBlock<'_>) -> Option<Action> {
    let id = Id::new(("ai-auto", block.target.key(), block.scope.key()));
    let requested: bool = ui.ctx().data(|d| d.get_temp(id)).unwrap_or(false);
    if requested {
        return None;
    }
    ui.ctx().data_mut(|d| d.insert_temp(id, true));
    Some(audit_action(block))
}

fn empty_body(ui: &mut Ui, block: &AiBlock<'_>, p: &Palette) {
    let label = if block.can_audit {
        text::AI_BLOCK_EMPTY
    } else {
        text::AUDIT_DISABLED
    };
    ui.label(RichText::new(label).color(p.text_muted));
}

pub fn status_badge(ui: &mut Ui, status: &AuditStatus, p: &Palette) {
    match status {
        AuditStatus::Queued => badge(ui, text::AUDIT_QUEUED, p.text_secondary),
        AuditStatus::Running => badge(ui, text::AUDIT_RUNNING, p.accent),
        AuditStatus::Done => badge(ui, text::AUDIT_DONE, p.ok),
        AuditStatus::Failed(_) => badge(ui, text::AUDIT_FAILED, p.critical),
        AuditStatus::Cancelled => badge(ui, text::AUDIT_CANCELLED, p.text_muted),
    }
}

pub fn cancel_button(ui: &mut Ui, report: &AuditReport) -> Option<Action> {
    if !report.is_running() {
        return None;
    }
    ui.small_button(text::AUDIT_CANCEL)
        .clicked()
        .then_some(Action::CancelAudit(report.id))
}

pub fn audit_status_label(status: &AuditStatus) -> &'static str {
    match status {
        AuditStatus::Queued => text::AUDIT_QUEUED,
        AuditStatus::Running => text::AUDIT_RUNNING,
        AuditStatus::Done => text::AUDIT_DONE,
        AuditStatus::Failed(_) => text::AUDIT_FAILED,
        AuditStatus::Cancelled => text::AUDIT_CANCELLED,
    }
}

pub fn report_body(ui: &mut Ui, report: &AuditReport, p: &Palette) {
    ui.horizontal_wrapped(|ui| {
        status_badge(ui, &report.status, p);
        ui.label(
            RichText::new(format!(
                "{} · {}",
                report.model,
                format::date_time(report.started_at)
            ))
            .small()
            .color(p.text_muted),
        );
    });
    ui.add_space(GAP_SMALL);
    match &report.status {
        AuditStatus::Failed(error) => {
            ui.label(RichText::new(error).color(p.critical));
        }
        AuditStatus::Queued | AuditStatus::Running => {
            ui.spinner();
        }
        AuditStatus::Done => markdown_lite(ui, &report.text),
        AuditStatus::Cancelled => {
            ui.label(RichText::new(text::AUDIT_CANCELLED_BODY).color(p.text_muted));
        }
    }
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

pub fn first_section(text: &str) -> String {
    let mut lines = Vec::new();
    let mut seen_heading = false;
    for line in text.lines() {
        if line.trim_start().starts_with('#') {
            if seen_heading {
                break;
            }
            seen_heading = true;
            continue;
        }
        if !line.trim().is_empty() {
            lines.push(strip_marks(line.trim()));
        }
    }
    lines.join(" ")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn first_section_returns_text_under_first_heading() {
        let text = "## Причина\n\nУпала **БД**.\nВторая строка.\n\n## Доказательства\n\n- x";
        assert_eq!(first_section(text), "Упала БД. Вторая строка.");
    }
}
