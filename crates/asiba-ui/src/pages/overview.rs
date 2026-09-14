use asiba_core::{AppState, ConnectionStatus, ServerState};
use asiba_modules::system::{self, SystemInfo};
use asiba_modules::{cpu, disk, memory, network};
use egui::{RichText, ScrollArea, Ui};

use super::{Action, Page};
use crate::components::{Table, page_title, panel, severity_color, status_label, tile};
use crate::format;
use crate::modules::attack_badge;
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
    let percent = |value: Option<f64>| {
        value
            .map(|v| format!("{v:.0}%"))
            .unwrap_or_else(|| "—".to_owned())
    };
    ui.horizontal_wrapped(|ui| {
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
        tile(
            ui,
            text::TILE_CPU,
            &percent(average(state, cpu::KEY_TOTAL)),
            None,
        );
        tile(
            ui,
            text::TILE_MEMORY,
            &percent(average(state, memory::KEY_USED_PCT)),
            None,
        );
        tile(
            ui,
            text::TILE_DISK,
            &percent(average(state, disk::KEY_ROOT_USED_PCT)),
            None,
        );
        let traffic = sum(state, network::KEY_RX_BPS).zip(sum(state, network::KEY_TX_BPS));
        let traffic_text = traffic
            .map(|(rx, tx)| {
                format!(
                    "{} / {}",
                    format::bytes_per_second(rx),
                    format::bytes_per_second(tx)
                )
            })
            .unwrap_or_else(|| "—".to_owned());
        tile(ui, text::TILE_TRAFFIC, &traffic_text, None);
        tile(ui, text::TILE_EVENTS, &state.events.len().to_string(), None);
    });
}

fn average(state: &AppState, key: &str) -> Option<f64> {
    let values: Vec<f64> = state
        .servers
        .values()
        .filter_map(|s| s.latest_value(key))
        .collect();
    if values.is_empty() {
        return None;
    }
    Some(values.iter().sum::<f64>() / values.len() as f64)
}

fn sum(state: &AppState, key: &str) -> Option<f64> {
    let values: Vec<f64> = state
        .servers
        .values()
        .filter_map(|s| s.latest_value(key))
        .collect();
    (!values.is_empty()).then(|| values.iter().sum())
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
        "CPU",
        "RAM",
        "DISK",
        "RX / TX",
        text::COL_UPTIME,
    ];
    Table::new("overview-servers", &columns).show(ui, |ui| {
        for server in state.servers.values() {
            if ui
                .link(RichText::new(server.spec.id.as_str()).strong())
                .clicked()
            {
                action = Some(Action::Navigate(Page::ServerDetail(server.spec.id.clone())));
            }
            ui.horizontal(|ui| {
                status_label(ui, &server.connection);
                attack_badge(ui, server);
            });
            ui.monospace(&server.spec.host);
            server_metrics(ui, server);
            ui.end_row();
        }
    });
    action
}

fn server_metrics(ui: &mut Ui, server: &ServerState) {
    let info = server.data::<SystemInfo>(system::ID);
    let percent = |key: &str| {
        server
            .latest_value(key)
            .map(|v| format!("{v:.0}%"))
            .unwrap_or_else(|| "—".to_owned())
    };
    ui.label(info.map(|i| i.os_name.as_str()).unwrap_or("—"));
    ui.monospace(percent(cpu::KEY_TOTAL));
    ui.monospace(percent(memory::KEY_USED_PCT));
    ui.monospace(percent(disk::KEY_ROOT_USED_PCT));
    let rx = server
        .latest_value(network::KEY_RX_BPS)
        .map(format::bytes_per_second);
    let tx = server
        .latest_value(network::KEY_TX_BPS)
        .map(format::bytes_per_second);
    ui.monospace(
        rx.zip(tx)
            .map(|(rx, tx)| format!("{rx} / {tx}"))
            .unwrap_or_else(|| "—".to_owned()),
    );
    ui.monospace(
        info.map(|i| i.uptime_human())
            .unwrap_or_else(|| "—".to_owned()),
    );
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
