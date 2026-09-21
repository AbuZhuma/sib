use asiba_core::{ModuleId, ServerState};
use asiba_modules::anomalies::{self, AnomaliesSnapshot, AttackSign};
use asiba_modules::security;
use egui::{RichText, Ui};

use super::{ModuleView, Tab, ViewAction, ViewShared, action_button};
use crate::components::{Table, TimeSeriesPlot, Unit, badge, severity_color, status_dot};
use crate::text;
use crate::theme::{GAP, Palette, SECONDARY_PLOT_HEIGHT};

const ATTACK_SIZE: f32 = 18.0;
const COUNTER_PAIR_WIDTH: f32 = 230.0;
const COUNTER_PAIRS_MAX: usize = 3;

pub struct AnomaliesView;

impl ModuleView for AnomaliesView {
    fn id(&self) -> ModuleId {
        anomalies::ID
    }

    fn title(&self) -> &'static str {
        text::MODULE_ANOMALIES
    }

    fn tab(&self) -> Tab {
        Tab::Anomalies
    }

    fn summary(&self, ui: &mut Ui, server: &ServerState, _shared: &ViewShared) {
        let Some(snapshot) = server.data::<AnomaliesSnapshot>(anomalies::ID) else {
            return;
        };
        headline(ui, snapshot);
        for sign in &snapshot.signs {
            sign_line(ui, sign);
        }
        ui.add_space(GAP);
        counters(ui, snapshot);
    }

    fn page(&self, ui: &mut Ui, server: &ServerState, shared: &ViewShared) -> Option<ViewAction> {
        let p = Palette::current(ui.ctx());
        let snapshot = server.data::<AnomaliesSnapshot>(anomalies::ID)?;
        headline(ui, snapshot);
        let mut action = signs_table(ui, snapshot);
        ui.add_space(GAP);
        counters(ui, snapshot);
        ui.add_space(GAP);
        plots(ui, server, &p);
        if let Some(next) = peers_table(ui, snapshot, shared) {
            action = Some(next);
        }
        ui.add_space(GAP);
        states(ui, snapshot);
        action
    }
}

pub fn attack_badge(ui: &mut Ui, server: &ServerState) {
    let p = Palette::current(ui.ctx());
    let Some(snapshot) = server.data::<AnomaliesSnapshot>(anomalies::ID) else {
        return;
    };
    if snapshot.is_under_attack() {
        badge(ui, text::ANOM_ATTACK, p.critical);
    } else if snapshot.has_signs() {
        badge(ui, text::ANOM_SUSPICIOUS, p.warning);
    }
}

fn headline(ui: &mut Ui, snapshot: &AnomaliesSnapshot) {
    let p = Palette::current(ui.ctx());
    let (label, color) = if snapshot.is_under_attack() {
        (text::ANOM_ATTACK, p.critical)
    } else if snapshot.has_signs() {
        (text::ANOM_SUSPICIOUS, p.warning)
    } else {
        (text::ANOM_CALM, p.ok)
    };
    ui.horizontal(|ui| {
        status_dot(ui, color);
        ui.label(RichText::new(label).size(ATTACK_SIZE).color(color));
    });
}

fn sign_line(ui: &mut Ui, sign: &AttackSign) {
    let p = Palette::current(ui.ctx());
    ui.horizontal_wrapped(|ui| {
        badge(ui, sign.kind.label(), severity_color(&p, sign.severity));
        ui.label(RichText::new(&sign.detail).color(p.text_secondary));
    });
}

fn signs_table(ui: &mut Ui, snapshot: &AnomaliesSnapshot) -> Option<ViewAction> {
    if snapshot.signs.is_empty() {
        return None;
    }
    let p = Palette::current(ui.ctx());
    let mut action = None;
    ui.add_space(GAP);
    let columns = [text::ANOM_SIGNS, text::COL_MESSAGE, "IP", ""];
    Table::new("anomalies-signs", &columns).show(ui, |ui| {
        for sign in &snapshot.signs {
            badge(ui, sign.kind.label(), severity_color(&p, sign.severity));
            ui.label(&sign.detail);
            ui.monospace(sign.peers.join(", "));
            ui.horizontal(|ui| {
                for peer in sign.peers.iter().take(3) {
                    let label = format!("{} {peer}", text::ACT_BAN);
                    if let Some(next) = action_button(ui, &label, security::SPEC_BAN, peer) {
                        action = Some(next);
                    }
                }
            });
            ui.end_row();
        }
    });
    action
}

fn counters(ui: &mut Ui, snapshot: &AnomaliesSnapshot) {
    let p = Palette::current(ui.ctx());
    let rate = |value: Option<f64>| {
        value
            .map(|v| format!("{v:.0}"))
            .unwrap_or_else(|| "-".to_owned())
    };
    let rates = snapshot.rates;
    let mut rows = vec![
        (text::ANOM_SYN_RECV, snapshot.syn_recv().to_string()),
        (text::ANOM_ESTAB, snapshot.established().to_string()),
        (text::ANOM_PEERS, snapshot.distinct_peers.to_string()),
        (
            text::ANOM_PPS,
            format!(
                "{} / {}",
                rate(rates.map(|r| r.pps_in)),
                rate(rates.map(|r| r.pps_out))
            ),
        ),
        (text::ANOM_NEW_CONN, rate(rates.map(|r| r.new_connections))),
        (
            text::ANOM_FAILED_CONN,
            rate(rates.map(|r| r.failed_connections)),
        ),
        (text::ANOM_SYNCOOKIES, rate(rates.map(|r| r.syncookies))),
        (text::ANOM_LISTEN_DROPS, rate(rates.map(|r| r.listen_drops))),
        (text::ANOM_UDP, rate(rates.map(|r| r.udp_in))),
    ];
    if let Some((count, max)) = snapshot.conntrack {
        rows.push((text::ANOM_CONNTRACK, format!("{count} / {max}")));
    }
    let pairs = ((ui.available_width() / COUNTER_PAIR_WIDTH) as usize).clamp(1, COUNTER_PAIRS_MAX);
    egui::Grid::new("anomalies-counters")
        .num_columns(pairs * 2)
        .spacing([16.0, 2.0])
        .show(ui, |ui| {
            for (index, (label, value)) in rows.iter().enumerate() {
                ui.label(RichText::new(*label).color(p.text_secondary));
                ui.monospace(value);
                if index % pairs == pairs - 1 {
                    ui.end_row();
                }
            }
        });
}

fn plots(ui: &mut Ui, server: &ServerState, p: &Palette) {
    let mut plot =
        TimeSeriesPlot::new("anomalies-conn-plot", Unit::Count).title(text::PLOT_ANOM_CONN);
    if let Some(series) = server.series.get(anomalies::KEY_SYN_RECV) {
        plot = plot.series(text::ANOM_PLOT_SYN, series, p.chart[3]);
    }
    if let Some(series) = server.series.get(anomalies::KEY_NEW_CONNECTIONS) {
        plot = plot.series(text::ANOM_PLOT_NEW, series, p.chart[0]);
    }
    plot.height(SECONDARY_PLOT_HEIGHT).show(ui);
    ui.add_space(GAP);
    let mut pps = TimeSeriesPlot::new("anomalies-pps-plot", Unit::Count).title(text::PLOT_ANOM_PPS);
    if let Some(series) = server.series.get(anomalies::KEY_PPS_IN) {
        pps = pps.series("pps in", series, p.chart[0]);
    }
    if let Some(series) = server.series.get(anomalies::KEY_PPS_OUT) {
        pps = pps.series("pps out", series, p.chart[2]);
    }
    pps.height(SECONDARY_PLOT_HEIGHT).show(ui);
    ui.add_space(GAP);
}

fn peers_table(
    ui: &mut Ui,
    snapshot: &AnomaliesSnapshot,
    shared: &ViewShared,
) -> Option<ViewAction> {
    let p = Palette::current(ui.ctx());
    let mut action = None;
    ui.label(
        RichText::new(text::ANOM_TOP_PEERS.to_uppercase())
            .small()
            .color(p.text_secondary),
    );
    let suspicious = snapshot.suspicious_peers();
    let columns = [
        "IP",
        text::ANOM_CONNECTIONS,
        "SYN-RECV",
        text::ANOM_SHARE,
        "",
    ];
    Table::new("anomalies-peers", &columns).show(ui, |ui| {
        for peer in &snapshot.top_peers {
            let color = if suspicious.contains(&peer.ip.as_str()) {
                p.critical
            } else {
                p.text
            };
            ui.label(RichText::new(&peer.ip).monospace().color(color));
            ui.monospace(shared.country(&peer.ip));
            ui.monospace(peer.connections.to_string());
            ui.monospace(peer.syn_recv.to_string());
            let share = if snapshot.total_connections > 0 {
                peer.connections as f64 * 100.0 / snapshot.total_connections as f64
            } else {
                0.0
            };
            ui.monospace(format!("{share:.0}%"));
            if let Some(next) = action_button(ui, text::ACT_BAN, security::SPEC_BAN, &peer.ip) {
                action = Some(next);
            }
            ui.end_row();
        }
    });
    action
}

fn states(ui: &mut Ui, snapshot: &AnomaliesSnapshot) {
    let p = Palette::current(ui.ctx());
    ui.horizontal_wrapped(|ui| {
        ui.label(RichText::new(text::ANOM_STATES).color(p.text_secondary));
        for (state, count) in &snapshot.states {
            let color = if state == "SYN-RECV" && *count > 0 {
                p.warning
            } else {
                p.text
            };
            ui.label(
                RichText::new(format!("{state} {count}"))
                    .monospace()
                    .color(color),
            );
        }
    });
}
