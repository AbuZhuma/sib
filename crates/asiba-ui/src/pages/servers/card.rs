use asiba_core::{AppState, Environment, ServerState};
use asiba_modules::system::{self, SystemInfo};
use asiba_modules::{cpu, disk, memory, network};
use egui::{Frame, Label, Margin, RichText, Sense, Stroke, Ui, UiBuilder, Vec2};

use crate::components::{badge, sparkline, status_label};
use crate::format;
use crate::modules::{attack_badge, short_label};
use crate::pages::overview::alert_counts;
use crate::text;
use crate::theme::{CARD_WIDTH, GAP, Palette};

const CARD_HEIGHT: f32 = 170.0;
const SPARKLINE_SIZE: Vec2 = Vec2::new(110.0, 30.0);

pub fn cards_per_row(available_width: f32) -> usize {
    let step = CARD_WIDTH + 2.0 * GAP + GAP;
    ((available_width + GAP) / step).floor().max(1.0) as usize
}

pub fn environment_label(environment: Environment) -> &'static str {
    match environment {
        Environment::Production => text::ENV_PRODUCTION,
        Environment::Staging => text::ENV_STAGING,
        Environment::Development => text::ENV_DEVELOPMENT,
        Environment::Other => text::ENV_OTHER,
    }
}

pub struct CardResponse {
    pub opened: bool,
    pub terminal: bool,
}

pub fn show(ui: &mut Ui, server: &ServerState, state: &AppState) -> CardResponse {
    let p = Palette::current(ui.ctx());
    let mut terminal = false;
    let builder = UiBuilder::new()
        .id_salt(("server-card", server.spec.id.as_str()))
        .sense(Sense::click());
    let scoped = ui.scope_builder(builder, |ui| {
        let hovered = ui.response().hovered();
        let stroke = if hovered { p.border_active } else { p.border };
        Frame::new()
            .fill(p.bg_panel)
            .stroke(Stroke::new(1.0, stroke))
            .inner_margin(Margin::same(GAP as i8))
            .show(ui, |ui| {
                ui.set_min_size(Vec2::new(CARD_WIDTH, CARD_HEIGHT));
                ui.set_max_width(CARD_WIDTH);
                ui.vertical(|ui| {
                    ui.set_width(CARD_WIDTH);
                    terminal = header(ui, server, state);
                    address_line(ui, server);
                    tags(ui, server);
                    ui.add_space(GAP);
                    body(ui, server);
                    modules_row(ui, server);
                });
            });
    });
    CardResponse {
        opened: scoped.response.clicked(),
        terminal,
    }
}

fn address_line(ui: &mut Ui, server: &ServerState) {
    let p = Palette::current(ui.ctx());
    let address = format!(
        "{}@{}:{}",
        server.spec.user, server.spec.host, server.spec.port
    );
    ui.add(Label::new(RichText::new(address).monospace().color(p.text_secondary)).truncate());
}

fn header(ui: &mut Ui, server: &ServerState, state: &AppState) -> bool {
    let p = Palette::current(ui.ctx());
    let mut terminal = false;
    ui.horizontal(|ui| {
        ui.label(RichText::new(server.spec.id.as_str()).heading());
        badge(
            ui,
            environment_label(server.spec.description.environment),
            p.text_secondary,
        );
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            terminal = ui
                .small_button(text::BTN_TERMINAL_SHORT)
                .on_hover_text(text::BTN_TERMINAL_HINT)
                .clicked();
            status_label(ui, &server.connection);
            attack_badge(ui, server);
            if state.active_alerts().any(|a| a.server == server.spec.id) {
                alert_counts(ui, state, server);
            }
        });
    });
    terminal
}

fn tags(ui: &mut Ui, server: &ServerState) {
    let p = Palette::current(ui.ctx());
    let description = &server.spec.description;
    let mut parts = Vec::new();
    if !description.project.is_empty() {
        parts.push(description.project.clone());
    }
    parts.extend(description.tags.iter().map(|t| format!("#{t}")));
    if parts.is_empty() {
        return;
    }
    ui.add(Label::new(RichText::new(parts.join("  ")).small().color(p.text_muted)).truncate());
}

fn modules_row(ui: &mut Ui, server: &ServerState) {
    let p = Palette::current(ui.ctx());
    let ids: Vec<asiba_core::ModuleId> = server.available_modules().collect();
    if ids.is_empty() {
        return;
    }
    ui.horizontal_wrapped(|ui| {
        ui.spacing_mut().item_spacing.x = 3.0;
        for id in ids {
            let label = RichText::new(short_label(id))
                .small()
                .monospace()
                .color(p.text_muted);
            ui.label(label).on_hover_text(id.0);
        }
    });
}

fn body(ui: &mut Ui, server: &ServerState) {
    let p = Palette::current(ui.ctx());
    let info = server.data::<SystemInfo>(system::ID);
    let Some(info) = info else {
        let project = &server.spec.description.project;
        let line = if project.is_empty() {
            text::DETAIL_NO_DATA
        } else {
            project.as_str()
        };
        ui.label(RichText::new(line).color(p.text_muted));
        return;
    };
    ui.add(Label::new(&info.os_name).truncate());
    ui.horizontal(|ui| {
        ui.vertical(|ui| {
            metric_line(
                ui,
                "CPU",
                server
                    .latest_value(cpu::KEY_TOTAL)
                    .map(|v| format!("{v:.0}%")),
            );
            metric_line(
                ui,
                "RAM",
                server
                    .latest_value(memory::KEY_USED_PCT)
                    .map(|v| format!("{v:.0}%")),
            );
            metric_line(
                ui,
                "Disk",
                server
                    .latest_value(disk::KEY_ROOT_USED_PCT)
                    .map(|v| format!("{v:.0}%")),
            );
        });
        ui.vertical(|ui| {
            sparkline(
                ui,
                server.series.get(cpu::KEY_TOTAL),
                SPARKLINE_SIZE,
                p.chart[0],
                Some(100.0),
            );
            let rx = server
                .latest_value(network::KEY_RX_BPS)
                .map(format::bytes_per_second);
            let tx = server
                .latest_value(network::KEY_TX_BPS)
                .map(format::bytes_per_second);
            if let (Some(rx), Some(tx)) = (rx, tx) {
                ui.monospace(
                    RichText::new(format!("↓{rx} ↑{tx}"))
                        .small()
                        .color(p.text_secondary),
                );
            }
        });
    });
    let ping = server
        .ping
        .as_ref()
        .and_then(|p| p.rtt_ms)
        .map(|v| format!("{v:.0} ms"))
        .unwrap_or_else(|| "-".to_owned());
    ui.monospace(
        RichText::new(format!("up {}  ping {ping}", info.uptime_human()))
            .small()
            .color(p.text_muted),
    );
}

fn metric_line(ui: &mut Ui, label: &str, value: Option<String>) {
    let p = Palette::current(ui.ctx());
    ui.horizontal(|ui| {
        ui.label(
            RichText::new(format!("{label:<5}"))
                .monospace()
                .color(p.text_secondary),
        );
        ui.monospace(value.unwrap_or_else(|| "-".to_owned()));
    });
}
