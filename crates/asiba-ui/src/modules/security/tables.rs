use asiba_core::ActionRequest;
use asiba_modules::security::{self, SecuritySnapshot};
use egui::{RichText, Ui};

use super::super::{ViewAction, ViewShared};
use crate::components::{Table, badge};
use crate::format;
use crate::text;
use crate::theme::Palette;

fn title(ui: &mut Ui, label: &str) {
    let p = Palette::current(ui.ctx());
    ui.label(
        RichText::new(label.to_uppercase())
            .small()
            .color(p.text_secondary),
    );
}

pub fn attackers(
    ui: &mut Ui,
    snapshot: &SecuritySnapshot,
    shared: &ViewShared,
) -> Option<ViewAction> {
    let p = Palette::current(ui.ctx());
    let mut action = None;
    title(ui, text::SEC_ATTACKERS);
    if snapshot.attackers.is_empty() {
        ui.label(RichText::new(text::SEC_NO_ATTACKERS).color(p.text_muted));
        return None;
    }
    let columns = [
        "IP",
        text::SEC_COUNTRY,
        text::SEC_ATTEMPTS,
        text::SEC_RECENT,
        text::SEC_USERS,
        text::SEC_FIRST,
        text::SEC_LAST,
        "",
    ];
    Table::new("security-attackers", &columns).show(ui, |ui| {
        for attacker in &snapshot.attackers {
            ui.monospace(&attacker.ip);
            ui.monospace(shared.country(&attacker.ip));
            ui.monospace(attacker.failures.to_string());
            let color = if attacker.is_brute_force() {
                p.critical
            } else {
                p.text
            };
            ui.label(
                RichText::new(attacker.recent_failures.to_string())
                    .monospace()
                    .color(color),
            );
            ui.label(RichText::new(attacker.users_label()).color(p.text_secondary));
            ui.monospace(format::date_time(attacker.first_at));
            ui.monospace(format::date_time(attacker.last_at));
            if snapshot.is_banned(&attacker.ip) {
                badge(ui, text::SEC_BANNED, p.text_muted);
            } else if ui.small_button(text::ACT_BAN).clicked() {
                action = Some(ViewAction::act(security::SPEC_BAN, &attacker.ip));
            }
            ui.end_row();
        }
    });
    action
}

pub fn bans(ui: &mut Ui, snapshot: &SecuritySnapshot) -> Option<ViewAction> {
    let p = Palette::current(ui.ctx());
    let mut action = None;
    title(ui, text::SEC_BANS);
    if snapshot.bans.is_empty() {
        ui.label(RichText::new(text::SEC_NO_BANS).color(p.text_muted));
        return None;
    }
    let columns = ["IP", text::SEC_BAN_SOURCE, text::SEC_BAN_EXPIRES, ""];
    Table::new("security-bans", &columns).show(ui, |ui| {
        for ban in &snapshot.bans {
            ui.monospace(&ban.ip);
            ui.label(RichText::new(&ban.source).color(p.text_secondary));
            ui.monospace(ban.expires.as_deref().unwrap_or("—"));
            if ui.small_button(text::ACT_UNBAN).clicked() {
                action = Some(ViewAction::Act {
                    spec: security::SPEC_UNBAN,
                    request: ActionRequest::new(security::ACTION_UNBAN, &ban.ip)
                        .with_argument(&ban.source),
                });
            }
            ui.end_row();
        }
    });
    action
}

pub fn logins(ui: &mut Ui, snapshot: &SecuritySnapshot) {
    let p = Palette::current(ui.ctx());
    title(ui, text::SEC_LOGINS);
    let columns = [
        text::COL_TIME,
        text::PROC_USER,
        text::USERS_FROM,
        text::SEC_METHOD,
    ];
    Table::new("security-logins", &columns).show(ui, |ui| {
        for login in &snapshot.logins {
            ui.monospace(format::date_time(login.at));
            let color = if login.is_root() { p.warning } else { p.text };
            ui.label(RichText::new(&login.user).monospace().color(color));
            ui.monospace(&login.from);
            ui.label(RichText::new(&login.method).color(p.text_secondary));
            ui.end_row();
        }
    });
}

pub fn sudo_calls(ui: &mut Ui, snapshot: &SecuritySnapshot) {
    let p = Palette::current(ui.ctx());
    title(ui, text::SEC_SUDO);
    let columns = [text::COL_TIME, text::PROC_USER, text::SEC_COMMAND, ""];
    Table::new("security-sudo", &columns).show(ui, |ui| {
        for call in &snapshot.sudo_calls {
            ui.monospace(format::date_time(call.at));
            ui.monospace(format!("{} → {}", call.user, call.target_user));
            ui.monospace(&call.command);
            if call.is_success {
                ui.label("");
            } else {
                badge(ui, text::SEC_DENIED, p.critical);
            }
            ui.end_row();
        }
    });
}
