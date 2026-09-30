use egui::{RichText, Ui};
use sib_core::{Availability, ServerState};

use crate::components::{Table, badge};
use crate::format;
use crate::text;
use crate::theme::Palette;

pub fn show(ui: &mut Ui, server: &ServerState) {
    let p = Palette::current(ui.ctx());
    if server.modules.is_empty() {
        ui.label(RichText::new(text::DETAIL_NO_DATA).color(p.text_muted));
        return;
    }
    let columns = [
        text::COL_MODULE,
        text::COL_STATUS,
        text::COL_UPDATED,
        text::COL_ERROR,
    ];
    Table::new("server-modules", &columns).show(ui, |ui| {
        for (id, state) in &server.modules {
            ui.monospace(id.0);
            let detail = availability(ui, &state.availability);
            let updated = state
                .last_collected
                .map(format::clock)
                .unwrap_or_else(|| "-".to_owned());
            ui.monospace(updated);
            match state.last_error.as_deref() {
                Some(error) => ui.label(RichText::new(error).color(p.critical)),
                None => ui.label(RichText::new(detail).small().color(p.text_muted)),
            };
            ui.end_row();
        }
    });
}

fn availability(ui: &mut Ui, availability: &Availability) -> String {
    let p = Palette::current(ui.ctx());
    let (label, color, detail) = match availability {
        Availability::Available => (text::AVAIL_AVAILABLE, p.ok, String::new()),
        Availability::Partial { missing } => (text::AVAIL_PARTIAL, p.warning, missing.join(", ")),
        Availability::Unavailable { reason } => {
            (text::AVAIL_UNAVAILABLE, p.offline, reason.clone())
        }
    };
    badge(ui, label, color);
    detail
}
