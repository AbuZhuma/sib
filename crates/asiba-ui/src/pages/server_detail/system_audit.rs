use asiba_incidents::{Area, AuditCheck, Outcome, SystemAudit, system_audit};
use egui::{Id, Label, RichText, Ui};

use super::DetailContext;
use crate::components::{badge, panel_with_controls};
use crate::text;
use crate::theme::{GAP, GAP_SMALL, Palette};

const SHOW_PASSED_KEY: &str = "system-audit-show-passed";

pub fn show(ui: &mut Ui, ctx: &DetailContext<'_>) {
    let p = Palette::current(ui.ctx());
    let audit = system_audit(ctx.server, ctx.state);
    let id = Id::new((SHOW_PASSED_KEY, ctx.server.spec.id.as_str()));
    let stored: bool = ui.ctx().data(|d| d.get_temp(id)).unwrap_or(false);
    let mut toggled = stored;
    panel_with_controls(
        ui,
        text::SYSTEM_AUDIT_TITLE,
        |ui| {
            ui.checkbox(&mut toggled, text::SYSTEM_AUDIT_SHOW_PASSED);
        },
        |ui| {
            let show_passed = stored;
            summary_line(ui, &audit, &p);
            ui.add_space(GAP_SMALL);
            for area in Area::ALL {
                let checks: Vec<&AuditCheck> = audit
                    .in_area(area)
                    .filter(|c| show_passed || c.is_problem())
                    .collect();
                if checks.is_empty() {
                    continue;
                }
                area_block(ui, area, &checks, &p);
            }
            if audit.problems().next().is_none() {
                ui.label(RichText::new(text::SYSTEM_AUDIT_CLEAN).color(p.ok));
            }
        },
    );
    ui.ctx().data_mut(|d| d.insert_temp(id, toggled));
    ui.add_space(GAP);
}

fn summary_line(ui: &mut Ui, audit: &SystemAudit, p: &Palette) {
    let counts = audit.counts();
    ui.horizontal_wrapped(|ui| {
        ui.label(
            RichText::new(format!(
                "{} {}",
                audit.checks.len(),
                text::SYSTEM_AUDIT_CHECKS
            ))
            .color(p.text_secondary),
        );
        badge(
            ui,
            &format!("{} {}", counts.passed, text::SYSTEM_AUDIT_PASSED),
            p.ok,
        );
        badge(
            ui,
            &format!("{} {}", counts.warnings, text::SYSTEM_AUDIT_WARNINGS),
            p.warning,
        );
        badge(
            ui,
            &format!("{} {}", counts.failures, text::SYSTEM_AUDIT_FAILURES),
            p.critical,
        );
        if counts.skipped > 0 {
            badge(
                ui,
                &format!("{} {}", counts.skipped, text::SYSTEM_AUDIT_SKIPPED),
                p.text_muted,
            );
        }
    });
}

fn area_block(ui: &mut Ui, area: Area, checks: &[&AuditCheck], p: &Palette) {
    ui.add_space(GAP_SMALL);
    ui.label(
        RichText::new(area.label().to_uppercase())
            .small()
            .color(p.text_secondary),
    );
    for check in checks {
        check_line(ui, check, p);
    }
}

fn check_line(ui: &mut Ui, check: &AuditCheck, p: &Palette) {
    let (label, color) = match check.outcome {
        Outcome::Pass => (text::SYSTEM_AUDIT_OK, p.ok),
        Outcome::Warn => (text::SYSTEM_AUDIT_WARN, p.warning),
        Outcome::Fail => (text::SYSTEM_AUDIT_FAIL, p.critical),
        Outcome::Skipped => (text::SYSTEM_AUDIT_SKIP, p.text_muted),
    };
    ui.horizontal_wrapped(|ui| {
        badge(ui, label, color);
        ui.label(RichText::new(&check.title).strong());
        ui.add(Label::new(RichText::new(&check.detail).color(p.text_secondary)).wrap());
    });
    if check.is_problem() {
        ui.add(Label::new(RichText::new(check.advice).small().color(p.text_muted)).wrap());
    }
}
