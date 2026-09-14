use asiba_core::{AppState, ConnectionStatus};
use asiba_modules::system::{self, SystemInfo};
use egui::{RichText, ScrollArea, Ui};

use super::{Action, Page};
use crate::components::{Table, page_title, panel, severity_color, status_label, tile};
use crate::format;
use crate::text;
use crate::theme::{GAP, Palette};

const EVENTS_SHOWN: usize = 30;

pub fn show(ui: &mut Ui, state: &AppState) -> Option<Action> {
    page_title(ui, text::OVERVIEW_TITLE);
    tiles(ui, state);
    ui.add_space(GAP);
    let mut action = None;
    ScrollArea::vertical().show(ui, |ui| {
        action = panel(ui, text::SECTION_SERVERS, |ui| servers_table(ui, state));
        ui.add_space(GAP);
        panel(ui, text::SECTION_EVENTS, |ui| events(ui, state));
    });
    action
}

fn tiles(ui: &mut Ui, state: &AppState) {
    let p = Palette::current(ui.ctx());
    let total = state.servers.len();
    let online = state.online_count();
    let offline = state
        .servers
        .values()
        .filter(|s| matches!(s.connection, ConnectionStatus::Offline { .. }))
        .count();
    ui.horizontal(|ui| {
        tile(ui, text::TILE_SERVERS, &total.to_string(), None);
        tile(
            ui,
            text::TILE_ONLINE,
            &online.to_string(),
            (online > 0).then_some(p.ok),
        );
        tile(
            ui,
            text::TILE_OFFLINE,
            &offline.to_string(),
            (offline > 0).then_some(p.critical),
        );
        tile(ui, text::TILE_EVENTS, &state.events.len().to_string(), None);
    });
}

fn servers_table(ui: &mut Ui, state: &AppState) -> Option<Action> {
    if state.servers.is_empty() {
        ui.label(text::EMPTY_SERVERS);
        return None;
    }
    let mut action = None;
    let columns = [
        text::COL_NAME,
        text::COL_STATUS,
        text::COL_HOST,
        text::COL_OS,
        text::COL_UPTIME,
        text::COL_LOAD,
    ];
    Table::new("overview-servers", &columns).show(ui, |ui| {
        for server in state.servers.values() {
            let info = server
                .snapshot(system::ID)
                .and_then(|s| s.downcast::<SystemInfo>());
            if ui
                .link(RichText::new(server.spec.id.as_str()).strong())
                .clicked()
            {
                action = Some(Action::Navigate(Page::ServerDetail(server.spec.id.clone())));
            }
            status_label(ui, &server.connection);
            ui.monospace(&server.spec.host);
            ui.label(info.map(|i| i.os_name.as_str()).unwrap_or("—"));
            ui.monospace(
                info.map(|i| i.uptime_human())
                    .unwrap_or_else(|| "—".to_owned()),
            );
            ui.monospace(
                info.map(|i| format!("{:.2}", i.load.one))
                    .unwrap_or_else(|| "—".to_owned()),
            );
            ui.end_row();
        }
    });
    action
}

fn events(ui: &mut Ui, state: &AppState) {
    if state.events.is_empty() {
        ui.label(text::EMPTY_EVENTS);
        return;
    }
    let p = Palette::current(ui.ctx());
    let columns = [
        text::COL_TIME,
        text::COL_SERVER,
        text::COL_MODULE,
        text::COL_MESSAGE,
    ];
    Table::new("overview-events", &columns).show(ui, |ui| {
        for event in state.events.iter().rev().take(EVENTS_SHOWN) {
            ui.monospace(format::clock(event.at));
            ui.label(event.server.as_ref().map(|s| s.as_str()).unwrap_or("—"));
            ui.label(event.module.0);
            ui.label(RichText::new(&event.message).color(severity_color(&p, event.severity)));
            ui.end_row();
        }
    });
}
