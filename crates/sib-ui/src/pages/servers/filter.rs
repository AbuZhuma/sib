use egui::{TextEdit, Ui};
use sib_core::{AppState, ConnectionStatus, Environment, ServerState};

use super::environment_label;
use crate::components::chip_value;
use crate::text;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum ViewMode {
    #[default]
    Grid,
    Table,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum StatusFilter {
    #[default]
    All,
    Online,
    Offline,
    Problems,
}

#[derive(Clone, Debug, Default)]
pub struct Filter {
    pub view: ViewMode,
    pub query: String,
    pub status: StatusFilter,
    pub environment: Option<Environment>,
}

impl Filter {
    pub fn matches(&self, server: &ServerState, state: &AppState) -> bool {
        self.matches_query(server) && self.matches_status(server, state) && self.matches_env(server)
    }

    fn matches_query(&self, server: &ServerState) -> bool {
        let needle = self.query.trim().to_lowercase();
        if needle.is_empty() {
            return true;
        }
        let description = &server.spec.description;
        server.spec.id.as_str().contains(&needle)
            || server.spec.host.to_lowercase().contains(&needle)
            || description.project.to_lowercase().contains(&needle)
            || description
                .tags
                .iter()
                .any(|t| t.to_lowercase().contains(&needle))
    }

    fn matches_status(&self, server: &ServerState, state: &AppState) -> bool {
        match self.status {
            StatusFilter::All => true,
            StatusFilter::Online => server.connection.is_online(),
            StatusFilter::Offline => !server.connection.is_online(),
            StatusFilter::Problems => {
                matches!(server.connection, ConnectionStatus::Offline { .. })
                    || state.active_alerts().any(|a| a.server == server.spec.id)
            }
        }
    }

    fn matches_env(&self, server: &ServerState) -> bool {
        self.environment
            .is_none_or(|env| server.spec.description.environment == env)
    }
}

pub fn toolbar(ui: &mut Ui, filter: &mut Filter, state: &AppState) {
    ui.horizontal_wrapped(|ui| {
        ui.add(
            TextEdit::singleline(&mut filter.query)
                .hint_text(text::SERVERS_SEARCH)
                .desired_width(200.0),
        );
        for (status, label) in [
            (StatusFilter::All, text::FILTER_ALL),
            (StatusFilter::Online, text::STATUS_ONLINE),
            (StatusFilter::Offline, text::STATUS_OFFLINE),
            (StatusFilter::Problems, text::FILTER_PROBLEMS),
        ] {
            chip_value(ui, &mut filter.status, status, label);
        }
        ui.separator();
        chip_value(ui, &mut filter.environment, None, text::FILTER_ALL);
        for env in environments_in_use(state) {
            chip_value(
                ui,
                &mut filter.environment,
                Some(env),
                environment_label(env),
            );
        }
        ui.separator();
        chip_value(ui, &mut filter.view, ViewMode::Grid, text::VIEW_GRID);
        chip_value(ui, &mut filter.view, ViewMode::Table, text::VIEW_TABLE);
    });
}

fn environments_in_use(state: &AppState) -> Vec<Environment> {
    let mut environments: Vec<Environment> = state
        .servers
        .values()
        .map(|s| s.spec.description.environment)
        .collect();
    environments.sort_by_key(|e| *e as u8);
    environments.dedup();
    environments
}
