use asiba_core::ServerState;
use egui::{RichText, Ui};

use crate::components::{badge, status_label};
use crate::modules::attack_badge;
use crate::pages::servers::environment_label;
use crate::pages::{Action, Page};
use crate::text;
use crate::theme::{GAP, Palette};

pub fn show(ui: &mut Ui, server: &ServerState) -> Option<Action> {
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
        let address = format!(
            "{}@{}:{}",
            server.spec.user, server.spec.host, server.spec.port
        );
        ui.monospace(RichText::new(address).color(p.text_secondary));
        status_label(ui, &server.connection);
        attack_badge(ui, server);
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

pub fn description(ui: &mut Ui, server: &ServerState) {
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
    let filled: Vec<_> = rows.iter().filter(|(_, v)| !v.is_empty()).collect();
    if filled.is_empty() {
        ui.label(RichText::new(text::DETAIL_NO_DESCRIPTION).color(p.text_muted));
        return;
    }
    egui::Grid::new("server-description")
        .num_columns(2)
        .spacing([16.0, 4.0])
        .show(ui, |ui| {
            for (label, value) in filled {
                ui.label(RichText::new(*label).color(p.text_secondary));
                ui.label(value);
                ui.end_row();
            }
        });
}
