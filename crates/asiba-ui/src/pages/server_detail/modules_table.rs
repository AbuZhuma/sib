use asiba_core::{Availability, ServerState};
use egui::{RichText, Ui};

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
            availability(ui, &state.availability);
            let updated = state
                .last_collected
                .map(format::clock)
                .unwrap_or_else(|| "—".to_owned());
            ui.monospace(updated);
            let error = state.last_error.as_deref().unwrap_or("");
            ui.label(RichText::new(error).color(p.critical));
            ui.end_row();
        }
    });
}

fn availability(ui: &mut Ui, availability: &Availability) {
    let p = Palette::current(ui.ctx());
    match availability {
        Availability::Available => badge(ui, text::AVAIL_AVAILABLE, p.ok),
        Availability::Partial { missing } => {
            badge(ui, text::AVAIL_PARTIAL, p.warning);
            ui.label(
                RichText::new(missing.join(", "))
                    .small()
                    .color(p.text_muted),
            );
        }
        Availability::Unavailable { reason } => {
            badge(ui, text::AVAIL_UNAVAILABLE, p.offline);
            ui.label(RichText::new(reason).small().color(p.text_muted));
        }
    }
}
