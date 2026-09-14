use asiba_core::{Availability, ConnectionStatus, ServerState};
use egui::{RichText, ScrollArea, Ui};

use super::servers::environment_label;
use super::{Action, Page};
use crate::components::{Table, badge, panel, status_label};
use crate::format;
use crate::modules::ModuleView;
use crate::text;
use crate::theme::{GAP, Palette};

pub fn show(ui: &mut Ui, server: &ServerState, views: &[Box<dyn ModuleView>]) -> Option<Action> {
    let mut action = header(ui, server);
    ScrollArea::vertical().show(ui, |ui| {
        if let Some(next) = panel(ui, text::DETAIL_SECTION_CONNECTION, |ui| {
            connection(ui, server)
        }) {
            action = Some(next);
        }
        ui.add_space(GAP);
        for view in views {
            if let Some(snapshot) = server.snapshot(view.id()) {
                panel(ui, view.title(), |ui| view.summary(ui, snapshot));
                ui.add_space(GAP);
            }
        }
        panel(ui, text::DETAIL_SECTION_MODULES, |ui| {
            modules_table(ui, server)
        });
        ui.add_space(GAP);
        panel(ui, text::DETAIL_SECTION_DESCRIPTION, |ui| {
            description(ui, server)
        });
    });
    action
}

fn header(ui: &mut Ui, server: &ServerState) -> Option<Action> {
    let p = Palette::current(ui.ctx());
    let mut action = None;
    ui.horizontal(|ui| {
        if ui.button(text::BTN_BACK).clicked() {
            action = Some(Action::Navigate(Page::Servers));
        }
        ui.label(RichText::new(server.spec.id.as_str()).heading());
        badge(
            ui,
            environment_label(server.spec.description.environment),
            p.text_secondary,
        );
        ui.monospace(
            RichText::new(format!(
                "{}@{}:{}",
                server.spec.user, server.spec.host, server.spec.port
            ))
            .color(p.text_secondary),
        );
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            if ui.button(text::BTN_DELETE).clicked() {
                action = Some(Action::AskDelete(server.spec.id.clone()));
            }
            if ui.button(text::BTN_EDIT).clicked() {
                action = Some(Action::OpenForm(Some(server.spec.id.clone())));
            }
            if ui.button(text::BTN_RECONNECT).clicked() {
                action = Some(Action::Reconnect(server.spec.id.clone()));
            }
        });
    });
    ui.add_space(GAP);
    action
}

fn connection(ui: &mut Ui, server: &ServerState) -> Option<Action> {
    let p = Palette::current(ui.ctx());
    status_label(ui, &server.connection);
    match &server.connection {
        ConnectionStatus::Online { since } => {
            ui.label(format!(
                "{} {}",
                text::DETAIL_ONLINE_SINCE,
                format::date_time(*since)
            ));
            None
        }
        ConnectionStatus::Offline { reason, retry_at } => {
            ui.label(RichText::new(reason).color(p.critical));
            let seconds = format::seconds_until(*retry_at);
            ui.label(format!("{} {seconds} с", text::DETAIL_RETRY_AT));
            None
        }
        ConnectionStatus::UntrustedHostKey {
            fingerprint,
            changed,
        } => {
            let warning = if *changed {
                text::TEST_CHANGED_KEY
            } else {
                text::TEST_UNKNOWN_KEY
            };
            ui.label(RichText::new(warning).color(p.warning));
            ui.monospace(fingerprint);
            ui.button(text::BTN_TRUST_KEY_SERVER)
                .clicked()
                .then(|| Action::TrustHostKey {
                    server: server.spec.id.clone(),
                    fingerprint: fingerprint.clone(),
                })
        }
        ConnectionStatus::Connecting => None,
    }
}

fn modules_table(ui: &mut Ui, server: &ServerState) {
    let p = Palette::current(ui.ctx());
    if server.modules.is_empty() {
        ui.label(RichText::new(text::DETAIL_NO_DATA).color(p.text_muted));
        return;
    }
    let columns = [
        text::COL_MODULE,
        text::COL_STATUS,
        text::COL_UPDATED,
        text::COL_ERROR,
    ];
    Table::new("server-modules", &columns).show(ui, |ui| {
        for (id, state) in &server.modules {
            ui.monospace(id.0);
            availability(ui, &state.availability);
            let updated = state
                .last_collected
                .map(format::clock)
                .unwrap_or_else(|| "—".to_owned());
            ui.monospace(updated);
            let error = state.last_error.as_deref().unwrap_or("");
            ui.label(RichText::new(error).color(p.critical));
            ui.end_row();
        }
    });
}

fn availability(ui: &mut Ui, availability: &Availability) {
    let p = Palette::current(ui.ctx());
    match availability {
        Availability::Available => badge(ui, text::AVAIL_AVAILABLE, p.ok),
        Availability::Partial { missing } => {
            badge(ui, text::AVAIL_PARTIAL, p.warning);
            ui.label(
                RichText::new(missing.join(", "))
                    .small()
                    .color(p.text_muted),
            );
        }
        Availability::Unavailable { reason } => {
            badge(ui, text::AVAIL_UNAVAILABLE, p.offline);
            ui.label(RichText::new(reason).small().color(p.text_muted));
        }
    }
}

fn description(ui: &mut Ui, server: &ServerState) {
    let p = Palette::current(ui.ctx());
    let d = &server.spec.description;
    let rows = [
        (text::FORM_PROJECT, d.project.clone()),
        (text::FORM_PURPOSE, d.purpose.clone()),
        (text::FORM_OWNER, d.owner.clone()),
        (text::FORM_TAGS, d.tags.join(", ")),
        (text::FORM_LINKS, d.links.join("\n")),
        (text::FORM_NOTES, d.notes.clone()),
    ];
    egui::Grid::new("server-description")
        .num_columns(2)
        .spacing([16.0, 4.0])
        .show(ui, |ui| {
            for (label, value) in rows.iter().filter(|(_, v)| !v.is_empty()) {
                ui.label(RichText::new(*label).color(p.text_secondary));
                ui.label(value);
                ui.end_row();
            }
        });
}
