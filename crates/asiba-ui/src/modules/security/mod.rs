mod tables;

use asiba_core::{ModuleId, ServerState};
use asiba_modules::security::{self, Category, CheckStatus, Grade, SecuritySnapshot, Weight};
use egui::{Label, RichText, Ui};

use super::{ModuleView, Tab, ViewAction, ViewShared};
use crate::components::{badge, status_dot};
use crate::text;
use crate::theme::{GAP, GAP_SMALL, Palette};

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

    fn summary(&self, ui: &mut Ui, server: &ServerState) {
        let Some(snapshot) = server.data::<SecuritySnapshot>(security::ID) else {
            return;
        };
        score_line(ui, snapshot);
        ui.add_space(GAP);
        problems_short(ui, snapshot);
        ui.add_space(GAP);
        counters(ui, snapshot);
    }

    fn page(&self, ui: &mut Ui, server: &ServerState, shared: &ViewShared) -> Option<ViewAction> {
        let snapshot = server.data::<SecuritySnapshot>(security::ID)?;
        score_line(ui, snapshot);
        ui.add_space(GAP);
        checklist(ui, snapshot);
        ui.add_space(GAP);
        let mut action = tables::attackers(ui, snapshot, shared);
        ui.add_space(GAP);
        if let Some(next) = tables::bans(ui, snapshot) {
            action = Some(next);
        }
        ui.add_space(GAP);
        tables::logins(ui, snapshot);
        ui.add_space(GAP);
        tables::sudo_calls(ui, snapshot);
        action
    }
}

fn grade_color(grade: Grade, p: &Palette) -> egui::Color32 {
    match grade {
        Grade::A => p.ok,
        Grade::B => p.ok,
        Grade::C => p.warning,
        Grade::D => p.warning,
        Grade::F => p.critical,
    }
}

fn score_line(ui: &mut Ui, snapshot: &SecuritySnapshot) {
    let p = Palette::current(ui.ctx());
    let checks = security::checks(snapshot);
    let score = security::score(&checks);
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

fn status_color(status: CheckStatus, p: &Palette) -> egui::Color32 {
    match status {
        CheckStatus::Pass => p.ok,
        CheckStatus::Warn => p.warning,
        CheckStatus::Fail => p.critical,
        CheckStatus::Unknown => p.text_muted,
    }
}

fn checklist(ui: &mut Ui, snapshot: &SecuritySnapshot) {
    let p = Palette::current(ui.ctx());
    let checks = security::checks(snapshot);
    for category in Category::ALL {
        let group: Vec<&security::Check> =
            checks.iter().filter(|c| c.category == category).collect();
        if group.is_empty() {
            continue;
        }
        let problems = group.iter().filter(|c| c.is_problem()).count();
        ui.add_space(GAP_SMALL);
        ui.horizontal(|ui| {
            ui.label(
                RichText::new(category.label().to_uppercase())
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
        for check in group {
            check_line(ui, check, &p);
        }
    }
}

fn check_line(ui: &mut Ui, check: &security::Check, p: &Palette) {
    ui.horizontal(|ui| {
        status_dot(ui, status_color(check.status, p));
        ui.label(check.label);
        if check.weight == Weight::High && check.is_problem() {
            badge(ui, text::SEC_HIGH, p.critical);
        }
        ui.add(Label::new(RichText::new(&check.detail).color(p.text_secondary)).truncate());
    });
}

fn problems_short(ui: &mut Ui, snapshot: &SecuritySnapshot) {
    let p = Palette::current(ui.ctx());
    let checks = security::checks(snapshot);
    let mut problems: Vec<&security::Check> = checks.iter().filter(|c| c.is_problem()).collect();
    problems.sort_by_key(|c| std::cmp::Reverse((c.status == CheckStatus::Fail, c.weight as u8)));
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
