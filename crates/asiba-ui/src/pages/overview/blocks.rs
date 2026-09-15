use asiba_core::{AppState, ServerState};
use asiba_modules::anomalies::{self, AnomaliesSnapshot};
use asiba_modules::deploy::{self, DeployState};
use asiba_modules::{cpu, disk, memory, network};
use egui::{RichText, Ui};

use super::{Action, Page};
use crate::components::{Table, badge, incident_line, panel, severity_color};
use crate::format;
use crate::modules::deploy_timeline;
use crate::text;
use crate::theme::{GAP, Palette};

const TOP_COUNT: usize = 5;

pub fn active_deploys(ui: &mut Ui, state: &AppState) -> Option<Action> {
    let p = Palette::current(ui.ctx());
    let rows: Vec<(&ServerState, &deploy::Deploy)> = state
        .servers
        .values()
        .filter_map(|s| s.data::<DeployState>(deploy::ID).map(|d| (s, d)))
        .flat_map(|(s, d)| d.snapshot.active().map(move |deploy| (s, deploy)))
        .collect();
    if rows.is_empty() {
        return None;
    }
    let mut action = None;
    panel(ui, text::SECTION_DEPLOYS, |ui| {
        for (server, deploy) in rows {
            ui.horizontal(|ui| {
                if ui.link(server.spec.id.as_str()).clicked() {
                    action = Some(Action::Navigate(Page::ServerDetail(server.spec.id.clone())));
                }
                ui.label(RichText::new(&deploy.project).strong());
                ui.label(RichText::new(deploy.source.label()).color(p.text_muted));
                deploy_timeline(ui, deploy, &p);
            });
        }
    });
    ui.add_space(GAP);
    action
}

pub fn active_incidents(ui: &mut Ui, state: &AppState) -> Option<Action> {
    let mut incidents: Vec<&asiba_core::Incident> = state.active_incidents().collect();
    if incidents.is_empty() {
        return None;
    }
    incidents.sort_by_key(|i| std::cmp::Reverse((i.severity, i.started_at)));
    let mut action = None;
    panel(ui, text::SECTION_INCIDENTS, |ui| {
        for incident in incidents {
            if incident_line(ui, incident, true) {
                action = Some(Action::Navigate(Page::ServerDetail(
                    incident.server.clone(),
                )));
            }
        }
    });
    ui.add_space(GAP);
    action
}

pub fn active_anomalies(ui: &mut Ui, state: &AppState) -> Option<Action> {
    let p = Palette::current(ui.ctx());
    let rows: Vec<(&ServerState, &AnomaliesSnapshot)> = state
        .servers
        .values()
        .filter_map(|s| s.data::<AnomaliesSnapshot>(anomalies::ID).map(|a| (s, a)))
        .filter(|(_, a)| a.has_signs())
        .collect();
    if rows.is_empty() {
        return None;
    }
    let mut action = None;
    panel(ui, text::SECTION_ANOMALIES, |ui| {
        for (server, snapshot) in rows {
            ui.horizontal_wrapped(|ui| {
                if ui.link(server.spec.id.as_str()).clicked() {
                    action = Some(Action::Navigate(Page::ServerDetail(server.spec.id.clone())));
                }
                for sign in &snapshot.signs {
                    badge(ui, sign.kind.label(), severity_color(&p, sign.severity));
                    ui.label(RichText::new(&sign.detail).color(p.text_secondary));
                }
            });
        }
    });
    ui.add_space(GAP);
    action
}

pub fn top_servers(ui: &mut Ui, state: &AppState) -> Option<Action> {
    if state.servers.len() < 2 {
        return None;
    }
    let mut action = None;
    panel(ui, text::SECTION_TOP, |ui| {
        let columns = [
            "",
            "CPU",
            "",
            "RAM",
            "",
            text::TILE_DISK_SHORT,
            "",
            text::TILE_TRAFFIC,
        ];
        let rankings = [
            ranking(state, cpu::KEY_TOTAL, percent),
            ranking(state, memory::KEY_USED_PCT, percent),
            ranking(state, disk::KEY_ROOT_USED_PCT, percent),
            ranking_traffic(state),
        ];
        Table::new("overview-top", &columns).show(ui, |ui| {
            for index in 0..TOP_COUNT {
                for ranking in &rankings {
                    match ranking.get(index) {
                        Some((server, value)) => {
                            if ui.link(server.as_str()).clicked() {
                                action = Some(Action::Navigate(Page::ServerDetail(server.clone())));
                            }
                            ui.monospace(value);
                        }
                        None => {
                            ui.label("");
                            ui.label("");
                        }
                    }
                }
                ui.end_row();
            }
        });
    });
    ui.add_space(GAP);
    action
}

type Ranked = Vec<(asiba_core::ServerId, String)>;

fn percent(value: f64) -> String {
    format!("{value:.0}%")
}

fn ranking(state: &AppState, key: &str, render: fn(f64) -> String) -> Ranked {
    let mut rows: Vec<(asiba_core::ServerId, f64)> = state
        .servers
        .values()
        .filter_map(|s| s.latest_value(key).map(|v| (s.spec.id.clone(), v)))
        .collect();
    rows.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
    rows.into_iter()
        .take(TOP_COUNT)
        .map(|(id, v)| (id, render(v)))
        .collect()
}

fn ranking_traffic(state: &AppState) -> Ranked {
    let mut rows: Vec<(asiba_core::ServerId, f64)> = state
        .servers
        .values()
        .filter_map(|s| {
            let rx = s.latest_value(network::KEY_RX_BPS)?;
            let tx = s.latest_value(network::KEY_TX_BPS)?;
            Some((s.spec.id.clone(), rx + tx))
        })
        .collect();
    rows.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
    rows.into_iter()
        .take(TOP_COUNT)
        .map(|(id, v)| (id, format::bytes_per_second(v)))
        .collect()
}
