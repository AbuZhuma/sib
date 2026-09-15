mod ai;
mod blocks;
mod tiles;

use asiba_core::{AppState, ServerState, Severity};
use asiba_modules::system::{self, SystemInfo};
use asiba_modules::{cpu, disk, memory, network};
use egui::{Id, RichText, Ui, Vec2};

use super::{Action, Page};
use crate::components::{
    MapState, Table, chip_value, page_title, panel, scroll, severity_color, status_label,
};
use crate::format;
use crate::modules::attack_badge;
use crate::text;
use crate::theme::{GAP, MINI_MAP_HEIGHT, Palette};

const EVENTS_SHOWN: usize = 40;
const EVENT_FILTER_KEY: &str = "overview-event-filter";

pub struct OverviewContext<'a> {
    pub state: &'a AppState,
    pub map: &'a mut MapState,
    pub ai_consent: bool,
    pub can_audit: bool,
}

pub fn show(ui: &mut Ui, ctx: OverviewContext<'_>) -> Option<Action> {
    let (state, map, can_audit) = (ctx.state, ctx.map, ctx.can_audit);
    page_title(ui, text::OVERVIEW_TITLE);
    tiles::show(ui, state);
    ui.add_space(GAP);
    let mut action = None;
    scroll::vertical().show(ui, |ui| {
        let mut set = |next: Option<Action>| {
            if next.is_some() {
                action = next;
            }
        };
        set(blocks::active_incidents(ui, state, can_audit));
        set(ai::show(ui, state, ctx.ai_consent, can_audit));
        set(blocks::active_deploys(ui, state));
        set(blocks::active_anomalies(ui, state));
        set(panel(ui, text::SECTION_SERVERS, |ui| {
            servers_table(ui, state)
        }));
        ui.add_space(GAP);
        set(blocks::top_servers(ui, state));
        set(mini_map(ui, state, map));
        panel(ui, text::SECTION_EVENTS, |ui| events(ui, state));
    });
    action
}

fn mini_map(ui: &mut Ui, state: &AppState, map: &mut MapState) -> Option<Action> {
    if !state.servers.values().any(|s| s.location.is_some()) {
        return None;
    }
    let clicked = panel(ui, text::SECTION_MAP, |ui| {
        let visible = ui.clip_rect().right() - ui.cursor().left();
        let size = Vec2::new(visible.min(ui.available_width()), MINI_MAP_HEIGHT);
        ui.allocate_ui(size, |ui| map.show(ui, state, false)).inner
    });
    ui.add_space(GAP);
    clicked.map(|id| Action::Navigate(Page::ServerDetail(id)))
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
        text::COL_ALERTS,
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
            alert_counts(ui, state, server);
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

pub fn alert_counts(ui: &mut Ui, state: &AppState, server: &ServerState) {
    let p = Palette::current(ui.ctx());
    let count = |severity: Severity| {
        state
            .active_alerts()
            .filter(|a| a.server == server.spec.id && a.severity == severity)
            .count()
    };
    let critical = count(Severity::Critical);
    let warning = count(Severity::Warning);
    ui.horizontal(|ui| {
        if critical > 0 {
            ui.label(
                RichText::new(critical.to_string())
                    .monospace()
                    .color(p.critical),
            );
        }
        if warning > 0 {
            ui.label(
                RichText::new(warning.to_string())
                    .monospace()
                    .color(p.warning),
            );
        }
        if critical + warning == 0 {
            ui.label(RichText::new("—").color(p.text_muted));
        }
    });
}

fn events(ui: &mut Ui, state: &AppState) {
    let p = Palette::current(ui.ctx());
    let id = Id::new(EVENT_FILTER_KEY);
    let mut minimum: Severity = ui.ctx().data(|d| d.get_temp(id)).unwrap_or(Severity::Info);
    ui.horizontal(|ui| {
        ui.label(RichText::new(text::EVENTS_LEVEL).color(p.text_secondary));
        for (level, label) in [
            (Severity::Info, text::SEVERITY_INFO),
            (Severity::Warning, text::SEVERITY_WARNING),
            (Severity::Critical, text::SEVERITY_CRITICAL),
        ] {
            chip_value(ui, &mut minimum, level, label);
        }
    });
    ui.ctx().data_mut(|d| d.insert_temp(id, minimum));
    let shown: Vec<_> = state
        .events
        .iter()
        .rev()
        .filter(|e| e.severity >= minimum)
        .take(EVENTS_SHOWN)
        .collect();
    if shown.is_empty() {
        ui.label(RichText::new(text::EMPTY_EVENTS).color(p.text_muted));
        return;
    }
    let columns = [
        text::COL_TIME,
        text::COL_SERVER,
        text::COL_MODULE,
        text::COL_MESSAGE,
    ];
    Table::new("overview-events", &columns).show(ui, |ui| {
        for event in shown {
            ui.monospace(format::clock(event.at));
            ui.label(event.server.as_ref().map(|s| s.as_str()).unwrap_or("—"));
            ui.label(event.module.0);
            ui.label(RichText::new(&event.message).color(severity_color(&p, event.severity)));
            ui.end_row();
        }
    });
}
