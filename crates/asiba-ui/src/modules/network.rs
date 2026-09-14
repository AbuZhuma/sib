use asiba_core::{ModuleId, ServerState};
use asiba_modules::network::{self, NetworkSnapshot};
use egui::{RichText, Ui, Vec2};

use super::{ModuleView, Tab, ViewAction};
use crate::components::{Table, TimeSeriesPlot, Unit, sparkline_fill};
use crate::format;
use crate::text;
use crate::theme::{GAP, Palette, SECONDARY_PLOT_HEIGHT, SUMMARY_RATE_HEIGHT};

const RATE_LABEL_WIDTH: f32 = 130.0;

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

    fn summary(&self, ui: &mut Ui, server: &ServerState) {
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

    fn page(&self, ui: &mut Ui, server: &ServerState) -> Option<ViewAction> {
        let p = Palette::current(ui.ctx());
        let snapshot = server.data::<NetworkSnapshot>(network::ID)?;
        let mut plot = TimeSeriesPlot::new("network-plot", Unit::BytesPerSecond);
        if let Some(series) = server.series.get(network::KEY_RX_BPS) {
            plot = plot.series("RX", series, p.chart[0]);
        }
        if let Some(series) = server.series.get(network::KEY_TX_BPS) {
            plot = plot.series("TX", series, p.chart[2]);
        }
        plot.show(ui);
        ui.add_space(GAP);
        if let Some(series) = server.series.get(asiba_engine::PING_SERIES_KEY) {
            TimeSeriesPlot::new("ping-plot", Unit::Milliseconds)
                .height(SECONDARY_PLOT_HEIGHT)
                .series(text::NET_PING, series, p.chart[1])
                .show(ui);
            ui.add_space(GAP);
        }
        if let Some(series) = server.series.get(network::KEY_ESTABLISHED) {
            TimeSeriesPlot::new("connections-plot", Unit::Count)
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
            .unwrap_or_else(|| "—".to_owned());
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
    Table::new("network-interfaces", &columns).show(ui, |ui| {
        for interface in &snapshot.interfaces {
            ui.monospace(&interface.name);
            let color = if interface.is_up() { p.ok } else { p.offline };
            ui.label(RichText::new(&interface.state).color(color));
            ui.monospace(interface.addresses.join(" "));
            ui.monospace(
                interface
                    .speed_mbps
                    .map(|s| format!("{s} Mb/s"))
                    .unwrap_or_else(|| "—".to_owned()),
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
            ui.end_row();
        }
    });
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
