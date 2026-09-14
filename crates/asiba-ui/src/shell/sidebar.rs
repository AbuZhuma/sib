use asiba_core::AppState;
use egui::{RichText, ScrollArea, Ui};

use crate::components::status::status_color;
use crate::components::{nav_item, status_dot};
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
    for (label, target) in items {
        let selected = is_selected(page, &target);
        if nav_item(ui, label, selected).clicked() {
            action = Some(Action::Navigate(target));
        }
    }
    ui.add_space(GAP);
    ui.separator();
    ScrollArea::vertical().show(ui, |ui| {
        for server in state.servers.values() {
            let id = &server.spec.id;
            let selected = matches!(page, Page::ServerDetail(current) if current == id);
            let color = status_color(&p, &server.connection);
            ui.horizontal(|ui| {
                status_dot(ui, color);
                if nav_item(ui, id.as_str(), selected).clicked() {
                    action = Some(Action::Navigate(Page::ServerDetail(id.clone())));
                }
            });
        }
    });
    action
}

fn is_selected(page: &Page, target: &Page) -> bool {
    match (page, target) {
        (Page::ServerDetail(_) | Page::ServerForm, Page::Servers) => true,
        (current, target) => current == target,
    }
}
