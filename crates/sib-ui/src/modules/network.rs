use egui::{RichText, Ui, Vec2};
use sib_core::{ModuleId, ServerState};
use sib_modules::network::{self, Interface, NetworkSnapshot};

use super::{ModuleView, Tab, ViewAction, ViewShared};
use crate::components::{
    Sort, SortColumn, SortKey, Table, TimeSeriesPlot, Unit, sort_rows, sparkline_fill,
};
use crate::format;
use crate::text;
use crate::theme::{GAP, Palette, SECONDARY_PLOT_HEIGHT, SUMMARY_RATE_HEIGHT};

const RATE_LABEL_WIDTH: f32 = 130.0;
const INTERFACE_SORTABLE: [SortColumn; 4] = [
    SortColumn::text(0),
    SortColumn::number(4),
    SortColumn::number(5),
    SortColumn::number(6),
];
const INTERFACE_DEFAULT_SORT: Sort = Sort::ascending(0);

pub struct NetworkView;

impl ModuleView for NetworkView {
    fn id(&self) -> ModuleId {
        network::ID
    }

    fn title(&self) -> &'static str {
        text::MODULE_NETWORK
    }

    fn tab(&self) -> Tab {
        Tab::Network
    }

    fn summary(&self, ui: &mut Ui, server: &ServerState, _shared: &ViewShared) {
        let p = Palette::current(ui.ctx());
        let Some(snapshot) = server.data::<NetworkSnapshot>(network::ID) else {
            return;
        };
        rate_row(ui, server, "RX", network::KEY_RX_BPS, p.chart[0]);
        rate_row(ui, server, "TX", network::KEY_TX_BPS, p.chart[2]);
        ui.horizontal_wrapped(|ui| {
            ui.monospace(
                RichText::new(snapshot.primary_addresses().join("  ")).color(p.text_secondary),
            );
        });
        let established = snapshot.connections.get("ESTAB").copied().unwrap_or(0);
        ui.monospace(format!("{} {established}", text::NET_ESTABLISHED));
        if let Some(ping) = &server.ping {
            let value = ping
                .rtt_ms
                .map(|v| format!("{v:.1} ms"))
                .unwrap_or_else(|| text::NET_PING_LOST.to_owned());
            ui.monospace(format!("{} {value}", text::NET_PING));
        }
    }

    fn page(&self, ui: &mut Ui, server: &ServerState, _shared: &ViewShared) -> Option<ViewAction> {
        let p = Palette::current(ui.ctx());
        let snapshot = server.data::<NetworkSnapshot>(network::ID)?;
        let mut plot =
            TimeSeriesPlot::new("network-plot", Unit::BytesPerSecond).title(text::PLOT_NETWORK);
        if let Some(series) = server.series.get(network::KEY_RX_BPS) {
            plot = plot.series("RX", series, p.chart[0]);
        }
        if let Some(series) = server.series.get(network::KEY_TX_BPS) {
            plot = plot.series("TX", series, p.chart[2]);
        }
        plot.show(ui);
        ui.add_space(GAP);
        if let Some(series) = server.series.get(sib_engine::PING_SERIES_KEY) {
            TimeSeriesPlot::new("ping-plot", Unit::Milliseconds)
                .title(text::PLOT_PING)
                .height(SECONDARY_PLOT_HEIGHT)
                .series(text::NET_PING, series, p.chart[1])
                .show(ui);
            ui.add_space(GAP);
        }
        if let Some(series) = server.series.get(network::KEY_ESTABLISHED) {
            TimeSeriesPlot::new("connections-plot", Unit::Count)
                .title(text::PLOT_CONNECTIONS)
                .height(SECONDARY_PLOT_HEIGHT)
                .series(text::NET_ESTABLISHED, series, p.chart[4])
                .show(ui);
            ui.add_space(GAP);
        }
        interfaces_table(ui, snapshot);
        ui.add_space(GAP);
        connections_table(ui, snapshot);
        None
    }
}

fn rate_row(ui: &mut Ui, server: &ServerState, label: &str, key: &str, color: egui::Color32) {
    let p = Palette::current(ui.ctx());
    ui.horizontal(|ui| {
        let value = server
            .latest_value(key)
            .map(format::bytes_per_second)
            .unwrap_or_else(|| "-".to_owned());
        ui.allocate_ui(Vec2::new(RATE_LABEL_WIDTH, SUMMARY_RATE_HEIGHT), |ui| {
            ui.vertical(|ui| {
                ui.label(RichText::new(label).color(p.text_secondary));
                ui.monospace(value);
            });
        });
        sparkline_fill(ui, server.series.get(key), SUMMARY_RATE_HEIGHT, color, None);
    });
}

fn interfaces_table(ui: &mut Ui, snapshot: &NetworkSnapshot) {
    let p = Palette::current(ui.ctx());
    let columns = [
        text::NET_INTERFACE,
        text::COL_STATUS,
        text::NET_ADDRESSES,
        text::NET_SPEED,
        "RX",
        "TX",
        text::NET_ERRORS,
    ];
    let table = Table::new("network-interfaces", &columns)
        .sortable(&INTERFACE_SORTABLE, INTERFACE_DEFAULT_SORT);
    table.show_sorted(ui, |ui, sort| {
        let mut rows: Vec<&Interface> = snapshot.interfaces.iter().collect();
        sort_rows(&mut rows, sort, |interface, column| {
            interface_key(interface, column)
        });
        for interface in rows {
            interface_cells(ui, interface, &p);
            ui.end_row();
        }
    });
}

fn interface_cells(ui: &mut Ui, interface: &Interface, p: &Palette) {
    ui.monospace(&interface.name);
    let color = if interface.is_up() { p.ok } else { p.offline };
    ui.label(RichText::new(&interface.state).color(color));
    ui.monospace(interface.addresses.join(" "));
    ui.monospace(
        interface
            .speed_mbps
            .map(|s| format!("{s} Mb/s"))
            .unwrap_or_else(|| "-".to_owned()),
    );
    match &interface.rates {
        Some(rates) => {
            ui.monospace(format::bytes_per_second(rates.rx_bps));
            ui.monospace(format::bytes_per_second(rates.tx_bps));
        }
        None => {
            ui.monospace(format::bytes(interface.rx.bytes));
            ui.monospace(format::bytes(interface.tx.bytes));
        }
    }
    let errors =
        interface.rx.errors + interface.tx.errors + interface.rx.drops + interface.tx.drops;
    let error_color = if errors > 0 { p.warning } else { p.text_muted };
    ui.label(
        RichText::new(errors.to_string())
            .color(error_color)
            .monospace(),
    );
}

fn interface_key(interface: &Interface, column: usize) -> SortKey {
    let rx = interface.rates.as_ref().map(|r| r.rx_bps);
    let tx = interface.rates.as_ref().map(|r| r.tx_bps);
    match column {
        0 => SortKey::text(&interface.name),
        4 => SortKey::number(rx.unwrap_or(interface.rx.bytes as f64)),
        5 => SortKey::number(tx.unwrap_or(interface.tx.bytes as f64)),
        _ => SortKey::number(
            (interface.rx.errors + interface.tx.errors + interface.rx.drops + interface.tx.drops)
                as f64,
        ),
    }
}

fn connections_table(ui: &mut Ui, snapshot: &NetworkSnapshot) {
    let p = Palette::current(ui.ctx());
    ui.label(
        RichText::new(text::NET_CONNECTIONS.to_uppercase())
            .small()
            .color(p.text_secondary),
    );
    ui.horizontal_wrapped(|ui| {
        for (state, count) in &snapshot.connections {
            ui.monospace(format!("{state} {count}"));
        }
        if let Some(gateway) = &snapshot.gateway {
            ui.monospace(
                RichText::new(format!("{} {gateway}", text::NET_GATEWAY)).color(p.text_secondary),
            );
        }
    });
}
