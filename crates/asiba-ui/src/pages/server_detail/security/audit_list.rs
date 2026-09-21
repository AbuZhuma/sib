use asiba_incidents::{Area, AuditCheck, SystemAudit};
use asiba_modules::security::{self, SecuritySnapshot};
use egui::{Label, RichText, Sense, Ui, UiBuilder};

use super::DetailContext;
use super::state::Subpage;
use crate::components::status_dot;
use crate::modules::security_view::{
    category_header, is_showing_passed, outcome_color, score_line, show_passed_toggle,
};
use crate::pages::Action;
use crate::text;
use crate::theme::{GAP, Palette};

pub fn show(
    ui: &mut Ui,
    ctx: &DetailContext<'_>,
    audit: &SystemAudit,
    subpage: &mut Subpage,
) -> Option<Action> {
    let p = Palette::current(ui.ctx());
    let server_id = ctx.server.spec.id.as_str();
    let snapshot = ctx.server.data::<SecuritySnapshot>(security::ID);
    ui.horizontal(|ui| {
        if let Some(snapshot) = snapshot {
            score_line(ui, snapshot, audit);
        }
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Min), |ui| {
            show_passed_toggle(ui, server_id);
        });
    });
    counts_line(ui, audit, &p);
    ui.add_space(GAP);
    let show_passed = is_showing_passed(ui.ctx(), server_id);
    if !show_passed && audit.problems().next().is_none() {
        ui.label(RichText::new(text::SEC_CLEAN).color(p.ok));
        return None;
    }
    for area in Area::ALL {
        let checks: Vec<&AuditCheck> = audit
            .in_area(area)
            .filter(|c| show_passed || c.is_problem())
            .collect();
        if checks.is_empty() {
            continue;
        }
        let problems = checks.iter().filter(|c| c.is_problem()).count();
        category_header(ui, area.label(), problems, &p);
        for check in checks {
            if check_row(ui, check, &p) {
                *subpage = Subpage::problem(check.key());
            }
        }
    }
    None
}

fn counts_line(ui: &mut Ui, audit: &SystemAudit, p: &Palette) {
    let counts = audit.counts();
    let mut detail = format!(
        "{} {}: {} {}, {} {}, {} {}",
        audit.checks.len(),
        text::SYSTEM_AUDIT_CHECKS,
        counts.passed,
        text::SYSTEM_AUDIT_PASSED,
        counts.warnings,
        text::SYSTEM_AUDIT_WARNINGS,
        counts.failures,
        text::SYSTEM_AUDIT_FAILURES
    );
    if counts.skipped > 0 {
        detail.push_str(&format!(
            ", {} {}",
            counts.skipped,
            text::SYSTEM_AUDIT_SKIPPED
        ));
    }
    ui.label(RichText::new(detail).small().color(p.text_secondary));
}

fn check_row(ui: &mut Ui, check: &AuditCheck, p: &Palette) -> bool {
    let builder = UiBuilder::new()
        .id_salt(("audit-row", check.key()))
        .sense(Sense::click());
    let response = ui
        .scope_builder(builder, |ui| {
            ui.set_min_width(ui.available_width());
            ui.horizontal(|ui| {
                status_dot(ui, outcome_color(check.outcome, p));
                let title = RichText::new(check.title());
                let title = if ui.response().hovered() {
                    title.underline()
                } else {
                    title
                };
                ui.add(Label::new(title).selectable(false));
                ui.add(
                    Label::new(RichText::new(&check.detail).color(p.text_secondary))
                        .truncate()
                        .selectable(false),
                );
            });
        })
        .response
        .on_hover_cursor(egui::CursorIcon::PointingHand)
        .on_hover_text(text::SEC_OPEN_PROBLEM);
    response.clicked()
}
