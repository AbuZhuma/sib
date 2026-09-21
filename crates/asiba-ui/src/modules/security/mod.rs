mod filter;
pub mod tables;

use asiba_core::{ModuleId, ServerState};
use asiba_incidents::{AuditCheck, Grade, Outcome, SystemAudit, security_score, system_audit};
use asiba_modules::security::{self, SecuritySnapshot};
use egui::{Label, RichText, Ui};

use super::{ModuleView, Tab, ViewShared};
use crate::components::status_dot;
use crate::text;
use crate::theme::{GAP, GAP_SMALL, Palette};

pub use filter::{is_showing_passed, toggle as show_passed_toggle};

const SCORE_SIZE: f32 = 30.0;
const SUMMARY_PROBLEMS: usize = 5;

pub struct SecurityView;

impl ModuleView for SecurityView {
    fn id(&self) -> ModuleId {
        security::ID
    }

    fn title(&self) -> &'static str {
        text::MODULE_SECURITY
    }

    fn tab(&self) -> Tab {
        Tab::Security
    }

    fn summary(&self, ui: &mut Ui, server: &ServerState, shared: &ViewShared) {
        let Some(snapshot) = server.data::<SecuritySnapshot>(security::ID) else {
            return;
        };
        let audit = system_audit(server, shared.state);
        score_line(ui, snapshot, &audit);
        ui.add_space(GAP);
        problems_short(ui, &audit);
        ui.add_space(GAP);
        counters(ui, snapshot);
    }
}

fn grade_color(grade: Grade, p: &Palette) -> egui::Color32 {
    match grade {
        Grade::A | Grade::B => p.ok,
        Grade::C | Grade::D => p.warning,
        Grade::F => p.critical,
    }
}

pub fn score_line(ui: &mut Ui, snapshot: &SecuritySnapshot, audit: &SystemAudit) {
    let p = Palette::current(ui.ctx());
    let score = security_score(audit.checks.iter());
    let color = grade_color(score.grade, &p);
    ui.horizontal(|ui| {
        ui.label(
            RichText::new(score.grade.letter())
                .monospace()
                .size(SCORE_SIZE)
                .color(color),
        );
        ui.vertical(|ui| {
            ui.label(
                RichText::new(format!("{}% · {}", score.percent, text::SEC_SCORE)).color(p.text),
            );
            let mut detail = format!("{} {} {}", score.passed, text::SEC_OF, score.known);
            if score.failed_high > 0 {
                detail.push_str(&format!(
                    ", {} {}",
                    score.failed_high,
                    text::SEC_CRITICAL_FAILS
                ));
            }
            if let Some(backend) = snapshot.ban_backend {
                detail.push_str(&format!(" · {} {}", text::SEC_BAN_BACKEND, backend.label()));
            }
            ui.label(RichText::new(detail).small().color(p.text_secondary));
        });
    });
    if !snapshot.is_root_view && !snapshot.sshd.is_known() {
        ui.label(RichText::new(text::SEC_NEEDS_SUDO).color(p.text_muted));
    }
}

pub fn outcome_color(outcome: Outcome, p: &Palette) -> egui::Color32 {
    match outcome {
        Outcome::Pass => p.ok,
        Outcome::Warn => p.warning,
        Outcome::Fail => p.critical,
        Outcome::Skipped => p.text_muted,
    }
}

pub fn category_header(ui: &mut Ui, label: &str, problems: usize, p: &Palette) {
    ui.add_space(GAP_SMALL);
    ui.horizontal(|ui| {
        ui.label(
            RichText::new(label.to_uppercase())
                .small()
                .color(p.text_secondary),
        );
        if problems > 0 {
            ui.label(
                RichText::new(format!("{problems} {}", text::SEC_PROBLEMS))
                    .small()
                    .color(p.warning),
            );
        }
    });
}

fn check_line(ui: &mut Ui, check: &AuditCheck, p: &Palette) {
    ui.horizontal(|ui| {
        status_dot(ui, outcome_color(check.outcome, p));
        ui.label(check.title());
        ui.add(Label::new(RichText::new(&check.detail).color(p.text_secondary)).truncate());
    });
}

fn problems_short(ui: &mut Ui, audit: &SystemAudit) {
    let p = Palette::current(ui.ctx());
    let mut problems: Vec<&AuditCheck> = audit.security().filter(|c| c.is_problem()).collect();
    problems.sort_by_key(|c| std::cmp::Reverse((c.outcome, c.weight)));
    for check in problems.iter().take(SUMMARY_PROBLEMS) {
        check_line(ui, check, &p);
    }
    let more = problems.len().saturating_sub(SUMMARY_PROBLEMS);
    if more > 0 {
        ui.label(
            RichText::new(format!("+{more} {}", text::SEC_MORE))
                .small()
                .color(p.text_muted),
        );
    }
}

fn counters(ui: &mut Ui, snapshot: &SecuritySnapshot) {
    let p = Palette::current(ui.ctx());
    ui.horizontal_wrapped(|ui| {
        ui.monospace(format!(
            "{} {}",
            snapshot.failed_logins,
            text::SEC_FAILED_24H
        ));
        let attacking = snapshot.brute_force_count();
        if attacking > 0 {
            ui.label(
                RichText::new(format!("{attacking} {}", text::SEC_ATTACKING_NOW))
                    .monospace()
                    .color(p.critical),
            );
        }
        ui.monospace(format!(
            "{} {}",
            snapshot.bans.len(),
            text::SEC_BANS.to_lowercase()
        ));
    });
    for attacker in snapshot.attackers.iter().take(5) {
        let color = if attacker.is_brute_force() {
            p.critical
        } else {
            p.text_secondary
        };
        ui.label(
            RichText::new(format!(
                "{}  {}  {}",
                attacker.ip,
                attacker.failures,
                attacker.users_label()
            ))
            .monospace()
            .color(color),
        );
    }
}
