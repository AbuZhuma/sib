use egui::{RichText, Ui};
use sib_core::ServerState;
use sib_modules::system::{self, SystemInfo};

use crate::components::{badge, status_label};
use crate::modules::attack_badge;
use crate::pages::servers::environment_label;
use crate::pages::{Action, Page};
use crate::text;
use crate::theme::{GAP, Palette};

pub fn show(ui: &mut Ui, server: &ServerState) -> Option<Action> {
    let p = Palette::current(ui.ctx());
    let mut action = None;
    ui.horizontal_wrapped(|ui| {
        if ui.button(text::BTN_BACK).clicked() {
            action = Some(Action::Navigate(Page::Servers));
        }
        ui.label(RichText::new(server.spec.id.as_str()).heading());
        badge(
            ui,
            environment_label(server.spec.description.environment),
            p.text_secondary,
        );
        let address = format!(
            "{}@{}:{}",
            server.spec.user, server.spec.host, server.spec.port
        );
        ui.monospace(RichText::new(address).color(p.text_secondary));
        status_label(ui, &server.connection);
        attack_badge(ui, server);
        ui.separator();
        buttons(ui, server, &mut action);
    });
    facts_line(ui, server);
    ui.add_space(GAP);
    action
}

fn buttons(ui: &mut Ui, server: &ServerState, action: &mut Option<Action>) {
    let id = &server.spec.id;
    if ui.button(text::BTN_RECONNECT).clicked() {
        *action = Some(Action::Reconnect(id.clone()));
    }
    if ui.button(text::BTN_EDIT).clicked() {
        *action = Some(Action::OpenForm(Some(id.clone())));
    }
    if ui.button(text::BTN_SERVER_FILE).clicked() {
        *action = Some(Action::OpenServerFile(id.clone()));
    }
    if ui.button(text::BTN_TERMINAL).clicked() {
        *action = Some(Action::OpenTerminal(id.clone()));
    }
    if ui.button(text::BTN_DELETE).clicked() {
        *action = Some(Action::AskDelete(id.clone()));
    }
}

fn facts_line(ui: &mut Ui, server: &ServerState) {
    let p = Palette::current(ui.ctx());
    let mut facts: Vec<String> = Vec::new();
    if let Some(info) = server.data::<SystemInfo>(system::ID) {
        facts.push(info.os_name.clone());
        facts.push(format!("{} {}", text::SYS_UPTIME, info.uptime_human()));
    }
    if let Some(location) = &server.location
        && !location.label.is_empty()
    {
        facts.push(location.label.clone());
    }
    if let Some(rtt) = server.ping.as_ref().and_then(|ping| ping.rtt_ms) {
        facts.push(format!("{} {rtt:.0} ms", text::NET_PING));
    }
    if facts.is_empty() {
        return;
    }
    ui.label(RichText::new(facts.join("  ·  ")).color(p.text_secondary));
}

pub fn has_description(server: &ServerState) -> bool {
    !filled_rows(server).is_empty()
}

fn filled_rows(server: &ServerState) -> Vec<(&'static str, String)> {
    let d = &server.spec.description;
    let rows = [
        (text::FORM_PROJECT, d.project.clone()),
        (text::FORM_PURPOSE, d.purpose.clone()),
        (text::FORM_OWNER, d.owner.clone()),
        (text::FORM_TAGS, d.tags.join(", ")),
        (text::FORM_LINKS, d.links.join("\n")),
        (text::FORM_NOTES, d.notes.clone()),
    ];
    rows.into_iter().filter(|(_, v)| !v.is_empty()).collect()
}

pub fn description(ui: &mut Ui, server: &ServerState) {
    let p = Palette::current(ui.ctx());
    egui::Grid::new("server-description")
        .num_columns(2)
        .spacing([16.0, 4.0])
        .show(ui, |ui| {
            for (label, value) in filled_rows(server) {
                ui.label(RichText::new(label).color(p.text_secondary));
                ui.label(value);
                ui.end_row();
            }
        });
}
