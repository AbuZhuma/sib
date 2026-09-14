use asiba_core::{ModuleId, ServerState};
use asiba_modules::users::{self, UsersSnapshot};
use egui::{RichText, Ui};

use super::{ModuleView, Tab};
use crate::components::{Table, badge};
use crate::text;
use crate::theme::{GAP, Palette};

pub struct UsersView;

impl ModuleView for UsersView {
    fn id(&self) -> ModuleId {
        users::ID
    }

    fn title(&self) -> &'static str {
        text::MODULE_USERS
    }

    fn tab(&self) -> Tab {
        Tab::Users
    }

    fn summary(&self, ui: &mut Ui, server: &ServerState) {
        let p = Palette::current(ui.ctx());
        let Some(snapshot) = server.data::<UsersSnapshot>(users::ID) else {
            return;
        };
        ui.monospace(format!(
            "{} {}",
            snapshot.sessions.len(),
            text::USERS_SESSIONS.to_lowercase()
        ));
        for session in snapshot.sessions.iter().take(5) {
            let from = session.from.as_deref().unwrap_or("local");
            ui.label(
                RichText::new(format!(
                    "{} {} {from} {}",
                    session.user, session.tty, session.since
                ))
                .monospace()
                .color(p.text_secondary),
            );
        }
    }

    fn page(&self, ui: &mut Ui, server: &ServerState) -> Option<super::ViewAction> {
        let snapshot = server.data::<UsersSnapshot>(users::ID)?;
        sessions(ui, snapshot);
        ui.add_space(GAP);
        logins(ui, snapshot);
        ui.add_space(GAP);
        accounts(ui, snapshot);
        None
    }
}

fn title(ui: &mut Ui, label: &str) {
    let p = Palette::current(ui.ctx());
    ui.label(
        RichText::new(label.to_uppercase())
            .small()
            .color(p.text_secondary),
    );
}

fn sessions(ui: &mut Ui, snapshot: &UsersSnapshot) {
    title(ui, text::USERS_SESSIONS);
    let columns = [text::PROC_USER, "TTY", text::USERS_FROM, text::USERS_SINCE];
    Table::new("users-sessions", &columns).show(ui, |ui| {
        for session in &snapshot.sessions {
            ui.monospace(&session.user);
            ui.monospace(&session.tty);
            ui.monospace(session.from.as_deref().unwrap_or("local"));
            ui.monospace(&session.since);
            ui.end_row();
        }
    });
}

fn logins(ui: &mut Ui, snapshot: &UsersSnapshot) {
    let p = Palette::current(ui.ctx());
    title(ui, text::USERS_LOGINS);
    let columns = [text::PROC_USER, "TTY", text::USERS_FROM, text::COL_TIME];
    Table::new("users-logins", &columns).show(ui, |ui| {
        for record in &snapshot.logins {
            ui.monospace(&record.user);
            ui.monospace(&record.tty);
            ui.monospace(&record.from);
            let color = if record.still_logged_in { p.ok } else { p.text };
            ui.label(RichText::new(&record.when).monospace().color(color));
            ui.end_row();
        }
    });
}

fn accounts(ui: &mut Ui, snapshot: &UsersSnapshot) {
    let p = Palette::current(ui.ctx());
    title(ui, text::USERS_ACCOUNTS);
    let columns = [
        text::PROC_USER,
        "UID",
        text::USERS_SHELL,
        text::USERS_SUDO,
        text::USERS_KEYS,
    ];
    Table::new("users-accounts", &columns).show(ui, |ui| {
        for account in &snapshot.accounts {
            ui.monospace(&account.name);
            ui.monospace(account.uid.to_string());
            ui.monospace(&account.shell);
            if account.is_sudoer {
                badge(ui, text::USERS_SUDO, p.warning);
            } else {
                ui.label("");
            }
            let keys = snapshot
                .authorized_keys
                .iter()
                .find(|(home, _)| *home == account.home)
                .map(|(_, c)| *c)
                .unwrap_or(0);
            ui.monospace(keys.to_string());
            ui.end_row();
        }
    });
}
