use asiba_core::{ModuleId, ServerState};
use asiba_modules::ports::{self, ListeningPort, PortsSnapshot};
use egui::{RichText, Ui};

use super::{ModuleView, Tab};
use crate::components::{Table, badge};
use crate::text;
use crate::theme::Palette;

pub struct PortsView;

impl ModuleView for PortsView {
    fn id(&self) -> ModuleId {
        ports::ID
    }

    fn title(&self) -> &'static str {
        text::MODULE_PORTS
    }

    fn tab(&self) -> Tab {
        Tab::Ports
    }

    fn summary(&self, ui: &mut Ui, server: &ServerState) {
        let p = Palette::current(ui.ctx());
        let Some(snapshot) = server.data::<PortsSnapshot>(ports::ID) else {
            return;
        };
        ui.horizontal_wrapped(|ui| {
            ui.monospace(format!("{} {}", snapshot.ports.len(), text::PORT_LISTENING));
            ui.monospace(format!(
                "{} {}",
                snapshot.public_ports().count(),
                text::PORT_PUBLIC
            ));
        });
        let exposed: Vec<&ListeningPort> = snapshot.exposed_without_firewall().collect();
        if !exposed.is_empty() {
            let list: Vec<String> = exposed.iter().map(|p| p.port.to_string()).collect();
            ui.label(
                RichText::new(format!("{}: {}", text::PORT_EXPOSED, list.join(", ")))
                    .color(p.warning),
            );
        }
        if snapshot.firewall.is_none() {
            ui.label(
                RichText::new(text::PORT_NO_FIREWALL)
                    .small()
                    .color(p.text_muted),
            );
        }
    }

    fn page(&self, ui: &mut Ui, server: &ServerState) -> Option<super::ViewAction> {
        let p = Palette::current(ui.ctx());
        let snapshot = server.data::<PortsSnapshot>(ports::ID)?;
        if let Some(firewall) = &snapshot.firewall {
            ui.label(
                RichText::new(format!("{}: {:?}", text::PORT_FIREWALL, firewall.backend))
                    .color(p.text_secondary),
            );
        }
        let columns = [
            text::PORT_PORT,
            text::PORT_PROTO,
            text::PORT_ADDRESS,
            text::PORT_PROCESS,
            text::PORT_CONNECTIONS,
            text::PORT_FIREWALL,
            text::PORT_REACHABLE,
        ];
        Table::new("ports-table", &columns).show(ui, |ui| {
            for port in &snapshot.ports {
                ui.monospace(RichText::new(port.port.to_string()).strong());
                ui.monospace(port.protocol.label());
                ui.monospace(&port.address);
                ui.monospace(port.process_label());
                ui.monospace(port.connections.to_string());
                firewall_cell(ui, port, &p);
                reachable_cell(ui, port, &p);
                ui.end_row();
            }
        });
        None
    }
}

fn firewall_cell(ui: &mut Ui, port: &ListeningPort, p: &Palette) {
    match port.firewall_allowed {
        Some(true) => badge(ui, text::PORT_ALLOWED, p.ok),
        Some(false) => badge(ui, text::PORT_BLOCKED, p.text_muted),
        None => {
            ui.label(RichText::new("—").color(p.text_muted));
        }
    }
}

fn reachable_cell(ui: &mut Ui, port: &ListeningPort, p: &Palette) {
    match port.reachable {
        Some(true) if port.firewall_allowed != Some(true) && port.is_wildcard() => {
            badge(ui, text::PORT_OPEN, p.warning)
        }
        Some(true) => badge(ui, text::PORT_OPEN, p.ok),
        Some(false) => badge(ui, text::PORT_CLOSED, p.text_muted),
        None => {
            ui.label(RichText::new("—").color(p.text_muted));
        }
    }
}
