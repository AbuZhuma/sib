mod tables;

use asiba_core::{ModuleId, ServerState};
use asiba_modules::security::{self, CheckStatus, SecuritySnapshot};
use egui::{RichText, Ui};

use super::{ModuleView, Tab, ViewAction, ViewShared};
use crate::components::status_dot;
use crate::text;
use crate::theme::{GAP, Palette};

const SCORE_SIZE: f32 = 26.0;

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
        checklist(ui, snapshot);
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

fn score_line(ui: &mut Ui, snapshot: &SecuritySnapshot) {
    let p = Palette::current(ui.ctx());
    let checks = security::checks(snapshot);
    let (passed, total) = security::score(&checks);
    let color = if passed == total {
        p.ok
    } else if passed * 2 >= total {
        p.warning
    } else {
        p.critical
    };
    ui.horizontal(|ui| {
        ui.label(
            RichText::new(format!("{passed}/{total}"))
                .monospace()
                .size(SCORE_SIZE)
                .color(color),
        );
        ui.label(RichText::new(text::SEC_SCORE).color(p.text_secondary));
        if let Some(backend) = snapshot.ban_backend {
            ui.label(
                RichText::new(format!("{} {}", text::SEC_BAN_BACKEND, backend.label()))
                    .color(p.text_muted),
            );
        }
    });
    if !snapshot.is_root_view && !snapshot.sshd.is_known() {
        ui.label(RichText::new(text::SEC_NEEDS_SUDO).color(p.text_muted));
    }
}

fn checklist(ui: &mut Ui, snapshot: &SecuritySnapshot) {
    let p = Palette::current(ui.ctx());
    for check in security::checks(snapshot) {
        let color = match check.status {
            CheckStatus::Pass => p.ok,
            CheckStatus::Warn => p.warning,
            CheckStatus::Fail => p.critical,
            CheckStatus::Unknown => p.text_muted,
        };
        ui.horizontal(|ui| {
            status_dot(ui, color);
            ui.label(check.label);
            ui.label(RichText::new(check.detail).color(p.text_secondary));
        });
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
