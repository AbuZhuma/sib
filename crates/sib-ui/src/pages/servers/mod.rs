mod card;
mod filter;
mod table;

use egui::{Id, Ui, Vec2};
use sib_core::{AppState, ServerState};

pub use card::environment_label;
use filter::{Filter, ViewMode};

use super::{Action, Page};
use crate::components::page_title;
use crate::components::scroll;
use crate::text;
use crate::theme::GAP;

const FILTER_KEY: &str = "servers-filter";

fn stored_filter(ui: &mut Ui, state: &AppState) -> Filter {
    let id = Id::new(FILTER_KEY);
    let mut filter: Filter = ui.ctx().data(|d| d.get_temp(id)).unwrap_or_default();
    filter::toolbar(ui, &mut filter, state);
    ui.ctx().data_mut(|d| d.insert_temp(id, filter.clone()));
    filter
}

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
    let filter = stored_filter(ui, state);
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
        let next = match filter.view {
            ViewMode::Grid => cards(ui, &servers, state),
            ViewMode::Table => {
                table::show(ui, &servers, state).map(|id| Action::Navigate(Page::ServerDetail(id)))
            }
        };
        if next.is_some() {
            action = next;
        }
    });
    action
}

fn cards(ui: &mut Ui, servers: &[&ServerState], state: &AppState) -> Option<Action> {
    let per_row = card::cards_per_row(ui.available_width());
    let mut action = None;
    for row in servers.chunks(per_row) {
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing = Vec2::splat(GAP);
            for server in row {
                let response = card::show(ui, server, state);
                if response.terminal {
                    action = Some(Action::OpenTerminal(server.spec.id.clone()));
                } else if response.opened {
                    action = Some(Action::Navigate(Page::ServerDetail(server.spec.id.clone())));
                }
            }
        });
        ui.add_space(GAP);
    }
    action
}
