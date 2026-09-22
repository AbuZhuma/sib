use asiba_core::{AppState, ServerId, ServerState};
use asiba_modules::system::{self, SystemInfo};
use asiba_modules::{cpu, disk, memory};
use egui::{RichText, Ui};

use super::environment_label;
use crate::components::{Table, badge, status_label};
use crate::modules::attack_badge;
use crate::pages::overview::alert_counts;
use crate::text;
use crate::theme::Palette;

pub fn show(ui: &mut Ui, servers: &[&ServerState], state: &AppState) -> Option<ServerId> {
    let p = Palette::current(ui.ctx());
    let mut clicked = None;
    Table::new("servers-table", &COLUMNS).show(ui, |ui| {
        for server in servers {
            if ui
                .link(RichText::new(server.spec.id.as_str()).strong())
                .clicked()
            {
                clicked = Some(server.spec.id.clone());
            }
            identity_cells(ui, server, &p);
            usage_cells(ui, server);
            alert_counts(ui, state, server);
            ui.end_row();
        }
    });
    clicked
}

const COLUMNS: [&str; 10] = [
    text::COL_NAME,
    text::COL_STATUS,
    text::FORM_ENVIRONMENT,
    text::COL_HOST,
    text::FORM_TAGS_SHORT,
    "CPU",
    "RAM",
    "DISK",
    text::COL_UPTIME,
    text::COL_ALERTS,
];

fn identity_cells(ui: &mut Ui, server: &ServerState, p: &Palette) {
    ui.horizontal(|ui| {
        status_label(ui, &server.connection);
        attack_badge(ui, server);
    });
    badge(
        ui,
        environment_label(server.spec.description.environment),
        p.text_secondary,
    );
    ui.monospace(&server.spec.host);
    ui.label(RichText::new(server.spec.description.tags.join(", ")).color(p.text_secondary));
}

fn usage_cells(ui: &mut Ui, server: &ServerState) {
    for key in [
        cpu::KEY_TOTAL,
        memory::KEY_USED_PCT,
        disk::KEY_ROOT_USED_PCT,
    ] {
        ui.monospace(
            server
                .latest_value(key)
                .map(|v| format!("{v:.0}%"))
                .unwrap_or_else(|| "-".to_owned()),
        );
    }
    ui.monospace(
        server
            .data::<SystemInfo>(system::ID)
            .map(|i| i.uptime_human())
            .unwrap_or_else(|| "-".to_owned()),
    );
}
