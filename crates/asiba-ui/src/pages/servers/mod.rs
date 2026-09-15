mod card;
mod filter;
mod table;

use asiba_core::{AppState, ServerState};
use egui::{Id, Ui, Vec2};

pub use card::environment_label;
use filter::{Filter, ViewMode};

use super::{Action, Page};
use crate::components::page_title;
use crate::components::scroll;
use crate::text;
use crate::theme::GAP;

const FILTER_KEY: &str = "servers-filter";

pub fn show(ui: &mut Ui, state: &AppState) -> Option<Action> {
    let mut action = None;
    ui.horizontal(|ui| {
        page_title(ui, text::SERVERS_TITLE);
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            if ui.button(text::BTN_ADD_SERVER).clicked() {
                action = Some(Action::OpenForm(None));
            }
        });
    });
    if state.servers.is_empty() {
        ui.label(text::EMPTY_SERVERS);
        return action;
    }
    let id = Id::new(FILTER_KEY);
    let mut filter: Filter = ui.ctx().data(|d| d.get_temp(id)).unwrap_or_default();
    filter::toolbar(ui, &mut filter, state);
    ui.ctx().data_mut(|d| d.insert_temp(id, filter.clone()));
    ui.add_space(GAP);
    let servers: Vec<&ServerState> = state
        .servers
        .values()
        .filter(|s| filter.matches(s, state))
        .collect();
    if servers.is_empty() {
        ui.label(text::SERVERS_NO_MATCH);
        return action;
    }
    scroll::vertical().show(ui, |ui| {
        let clicked = match filter.view {
            ViewMode::Grid => cards(ui, &servers, state),
            ViewMode::Table => table::show(ui, &servers, state),
        };
        if let Some(id) = clicked {
            action = Some(Action::Navigate(Page::ServerDetail(id)));
        }
    });
    action
}

fn cards(ui: &mut Ui, servers: &[&ServerState], state: &AppState) -> Option<asiba_core::ServerId> {
    let per_row = card::cards_per_row(ui.available_width());
    let mut clicked = None;
    for row in servers.chunks(per_row) {
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing = Vec2::splat(GAP);
            for server in row {
                if card::show(ui, server, state).clicked() {
                    clicked = Some(server.spec.id.clone());
                }
            }
        });
        ui.add_space(GAP);
    }
    clicked
}
