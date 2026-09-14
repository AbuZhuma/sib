use asiba_core::ServerState;
use egui::{CollapsingHeader, RichText, Ui};

use crate::components::{Table, severity_color};
use crate::format;
use crate::text;
use crate::theme::{GAP, Palette};

const SHOWN: usize = 50;

pub fn show(ui: &mut Ui, server: &ServerState) {
    let p = Palette::current(ui.ctx());
    ui.add_space(GAP);
    let title = format!(
        "{} ({})",
        text::DETAIL_SECTION_EVENTS,
        server.recent_events.len()
    );
    CollapsingHeader::new(RichText::new(title).small().color(p.text_secondary))
        .id_salt(("server-events", server.spec.id.as_str()))
        .show(ui, |ui| {
            if server.recent_events.is_empty() {
                ui.label(RichText::new(text::EMPTY_EVENTS).color(p.text_muted));
                return;
            }
            let columns = [text::COL_TIME, text::COL_MODULE, text::COL_MESSAGE];
            Table::new("server-events", &columns).show(ui, |ui| {
                for event in server.recent_events.iter().rev().take(SHOWN) {
                    ui.monospace(format::clock(event.at));
                    ui.label(event.module.0);
                    ui.label(
                        RichText::new(&event.message).color(severity_color(&p, event.severity)),
                    );
                    ui.end_row();
                }
            });
        });
}
