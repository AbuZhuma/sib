use asiba_core::{AppState, ServerId};
use egui::{RichText, Ui};

use crate::components::status::status_color;
use crate::components::{nav_item, scroll, status_dot};
use crate::pages::{Action, Page};
use crate::text;
use crate::theme::{GAP, Palette};

pub fn sidebar(ui: &mut Ui, page: &Page, state: &AppState) -> Option<Action> {
    let p = Palette::current(ui.ctx());
    let mut action = None;
    ui.add_space(GAP);
    ui.label(RichText::new(text::APP_NAME).heading().color(p.text));
    ui.add_space(GAP);
    let items = [
        (text::NAV_OVERVIEW, Page::Overview),
        (text::NAV_SERVERS, Page::Servers),
        (text::NAV_ALERTS, Page::Alerts),
        (text::NAV_MAP, Page::Map),
        (text::NAV_SETTINGS, Page::Settings),
    ];
    let active_alerts = state.active_alerts().count();
    for (label, target) in items {
        let selected = is_selected(page, &target);
        let label = if target == Page::Alerts && active_alerts > 0 {
            format!("{label}  {active_alerts}")
        } else {
            label.to_owned()
        };
        if nav_item(ui, &label, selected).clicked() {
            action = Some(Action::Navigate(target));
        }
    }
    ui.add_space(GAP);
    ui.separator();
    if let Some(clicked) = server_list(ui, page, state, &p) {
        action = Some(Action::Navigate(Page::ServerDetail(clicked)));
    }
    action
}

fn server_list(ui: &mut Ui, page: &Page, state: &AppState, p: &Palette) -> Option<ServerId> {
    let mut clicked = None;
    scroll::vertical().show(ui, |ui| {
        for server in state.servers.values() {
            let id = &server.spec.id;
            let selected = matches!(page, Page::ServerDetail(current) if current == id);
            let color = status_color(p, &server.connection);
            ui.horizontal(|ui| {
                status_dot(ui, color);
                if nav_item(ui, id.as_str(), selected).clicked() {
                    clicked = Some(id.clone());
                }
            });
        }
    });
    clicked
}

fn is_selected(page: &Page, target: &Page) -> bool {
    match (page, target) {
        (Page::ServerDetail(_) | Page::ServerForm, Page::Servers) => true,
        (current, target) => current == target,
    }
}
