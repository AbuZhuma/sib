use asiba_core::{AppState, ConnectionStatus, Severity};
use asiba_modules::{cpu, disk, memory, network};
use egui::Ui;

use crate::components::{TileSpec, tile_grid};
use crate::format;
use crate::text;
use crate::theme::Palette;

pub fn show(ui: &mut Ui, state: &AppState) {
    let p = Palette::current(ui.ctx());
    let mut tiles = fleet_tiles(state, &p);
    tiles.extend(usage_tiles(state));
    tile_grid(ui, &tiles);
}

fn fleet_tiles(state: &AppState, p: &Palette) -> Vec<TileSpec> {
    let total = state.servers.len();
    let online = state.online_count();
    let offline = state
        .servers
        .values()
        .filter(|s| matches!(s.connection, ConnectionStatus::Offline { .. }))
        .count();
    let critical = state
        .active_alerts()
        .filter(|a| a.severity == Severity::Critical)
        .count();
    let alerts = state.active_alerts().count();
    vec![
        spec(text::TILE_SERVERS, total.to_string(), None),
        spec(
            text::TILE_ONLINE,
            online.to_string(),
            (online > 0).then_some(p.ok),
        ),
        spec(
            text::TILE_OFFLINE,
            offline.to_string(),
            (offline > 0).then_some(p.critical),
        ),
        spec(
            text::TILE_ALERTS,
            alerts.to_string(),
            alert_color(p, alerts, critical),
        ),
    ]
}

fn usage_tiles(state: &AppState) -> Vec<TileSpec> {
    let percent = |key: &str| {
        average(state, key)
            .map(|v| format!("{v:.0}%"))
            .unwrap_or_else(|| "-".to_owned())
    };
    let traffic = sum(state, network::KEY_RX_BPS)
        .zip(sum(state, network::KEY_TX_BPS))
        .map(|(rx, tx)| {
            format!(
                "{} / {}",
                format::bytes_per_second(rx),
                format::bytes_per_second(tx)
            )
        })
        .unwrap_or_else(|| "-".to_owned());
    vec![
        spec(text::TILE_CPU, percent(cpu::KEY_TOTAL), None),
        spec(text::TILE_MEMORY, percent(memory::KEY_USED_PCT), None),
        spec(text::TILE_DISK, percent(disk::KEY_ROOT_USED_PCT), None),
        spec(text::TILE_TRAFFIC, traffic, None),
    ]
}

fn spec(label: &'static str, value: String, color: Option<egui::Color32>) -> TileSpec {
    TileSpec {
        label,
        value,
        color,
    }
}

fn alert_color(p: &Palette, alerts: usize, critical: usize) -> Option<egui::Color32> {
    if critical > 0 {
        Some(p.critical)
    } else if alerts > 0 {
        Some(p.warning)
    } else {
        None
    }
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
