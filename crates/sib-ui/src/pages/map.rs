use egui::{RichText, Ui};
use sib_core::AppState;

use super::{Action, Page};
use crate::components::{MapState, page_title, status_dot};
use crate::text;
use crate::theme::{GAP, Palette};

pub fn show(ui: &mut Ui, state: &AppState, map: &mut MapState) -> Option<Action> {
    let p = Palette::current(ui.ctx());
    page_title(ui, text::MAP_TITLE);
    let located = state
        .servers
        .values()
        .filter(|s| s.location.is_some())
        .count();
    if located == 0 {
        ui.label(RichText::new(text::MAP_EMPTY).color(p.text_muted));
    }
    ui.horizontal_wrapped(|ui| {
        if state.self_location.is_some() {
            status_dot(ui, p.text);
            ui.label(RichText::new(text::MAP_LEGEND_HOME).color(p.text_secondary));
        }
        for server in state.servers.values().filter(|s| s.location.is_some()) {
            let color = crate::components::status::status_color(&p, &server.connection);
            status_dot(ui, color);
            ui.label(RichText::new(server.spec.id.as_str()).color(p.text_secondary));
        }
    });
    ui.add_space(GAP);
    map.show(ui, state, true)
        .map(|id| Action::Navigate(Page::ServerDetail(id)))
}
