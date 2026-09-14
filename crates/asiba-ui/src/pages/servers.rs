use asiba_core::{AppState, Environment, ServerState};
use asiba_modules::system::{self, SystemInfo};
use egui::{Frame, Margin, RichText, ScrollArea, Sense, Stroke, Ui, Vec2};

use super::{Action, Page};
use crate::components::{badge, page_title, status_label};
use crate::text;
use crate::theme::{CARD_WIDTH, GAP, Palette};

const CARD_HEIGHT: f32 = 120.0;

pub fn show(ui: &mut Ui, state: &AppState) -> Option<Action> {
    let mut action = None;
    ui.horizontal(|ui| {
        page_title(ui, text::SERVERS_TITLE);
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            if ui.button(text::BTN_ADD_SERVER).clicked() {
                action = Some(Action::OpenForm(None));
            }
        });
    });
    if state.servers.is_empty() {
        ui.label(text::EMPTY_SERVERS);
        return action;
    }
    ScrollArea::vertical().show(ui, |ui| {
        ui.horizontal_wrapped(|ui| {
            ui.spacing_mut().item_spacing = Vec2::splat(GAP);
            for server in state.servers.values() {
                if card(ui, server).clicked() {
                    action = Some(Action::Navigate(Page::ServerDetail(server.spec.id.clone())));
                }
            }
        });
    });
    action
}

pub fn environment_label(environment: Environment) -> &'static str {
    match environment {
        Environment::Production => text::ENV_PRODUCTION,
        Environment::Staging => text::ENV_STAGING,
        Environment::Development => text::ENV_DEVELOPMENT,
        Environment::Other => text::ENV_OTHER,
    }
}

fn card(ui: &mut Ui, server: &ServerState) -> egui::Response {
    let p = Palette::current(ui.ctx());
    let info = server
        .snapshot(system::ID)
        .and_then(|s| s.downcast::<SystemInfo>());
    let response = Frame::new()
        .fill(p.bg_panel)
        .stroke(Stroke::new(1.0, p.border))
        .inner_margin(Margin::same(GAP as i8))
        .show(ui, |ui| {
            ui.set_min_size(Vec2::new(CARD_WIDTH, CARD_HEIGHT));
            ui.set_max_width(CARD_WIDTH);
            header(ui, server);
            ui.monospace(
                RichText::new(format!(
                    "{}@{}:{}",
                    server.spec.user, server.spec.host, server.spec.port
                ))
                .color(p.text_secondary),
            );
            ui.add_space(GAP);
            body(ui, server, info);
        })
        .response;
    let hover = response.interact(Sense::click());
    if hover.hovered() {
        ui.painter().rect_stroke(
            response.rect,
            0.0,
            Stroke::new(1.0, p.border_active),
            egui::StrokeKind::Inside,
        );
    }
    hover
}

fn header(ui: &mut Ui, server: &ServerState) {
    let p = Palette::current(ui.ctx());
    ui.horizontal(|ui| {
        ui.label(RichText::new(server.spec.id.as_str()).heading());
        badge(
            ui,
            environment_label(server.spec.description.environment),
            p.text_secondary,
        );
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            status_label(ui, &server.connection);
        });
    });
}

fn body(ui: &mut Ui, server: &ServerState, info: Option<&SystemInfo>) {
    let p = Palette::current(ui.ctx());
    match info {
        Some(info) => {
            ui.label(&info.os_name);
            ui.horizontal(|ui| {
                ui.monospace(format!("up {}", info.uptime_human()));
                ui.monospace(format!("load {:.2}", info.load.one));
                ui.monospace(format!("{} cpu", info.cpu_cores));
            });
        }
        None => {
            let project = &server.spec.description.project;
            let line = if project.is_empty() {
                text::DETAIL_NO_DATA
            } else {
                project.as_str()
            };
            ui.label(RichText::new(line).color(p.text_muted));
        }
    }
    let modules = server.available_modules().count();
    ui.label(
        RichText::new(format!("{modules} модулей"))
            .small()
            .color(p.text_muted),
    );
}
