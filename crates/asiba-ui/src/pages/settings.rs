use asiba_config::{AppConfig, Paths, ThemeChoice};
use asiba_core::AppState;
use egui::{Grid, RichText, Ui};

use super::Action;
use crate::components::{chip_value, page_title, panel, scroll};
use crate::text;
use crate::theme::{GAP, Palette};

pub struct SettingsContext<'a> {
    pub paths: &'a Paths,
    pub config: &'a AppConfig,
    pub state: &'a AppState,
}

pub fn show(ui: &mut Ui, ctx: &SettingsContext<'_>) -> Option<Action> {
    page_title(ui, text::SETTINGS_TITLE);
    let mut action = None;
    scroll::vertical().show(ui, |ui| {
        action = sections(ui, ctx);
    });
    action
}

fn sections(ui: &mut Ui, ctx: &SettingsContext<'_>) -> Option<Action> {
    let (paths, config) = (ctx.paths, ctx.config);
    let mut action = None;
    panel(ui, text::SETTINGS_SECTION_APPEARANCE, |ui| {
        action = theme_picker(ui, config.theme);
    });
    ui.add_space(GAP);
    panel(ui, text::SETTINGS_SECTION_PATHS, |ui| {
        paths_grid(ui, paths, config)
    });
    ui.add_space(GAP);
    let collection = panel(ui, text::SETTINGS_SECTION_COLLECTION, |ui| {
        super::collection_settings::show(ui, config)
    });
    if collection.is_some() {
        action = collection;
    }
    ui.add_space(GAP);
    let rules = panel(ui, text::SETTINGS_SECTION_ALERTS, |ui| {
        super::alert_rules::show(ui, config)
    });
    if rules.is_some() {
        action = rules;
    }
    ui.add_space(GAP);
    panel(ui, text::SETTINGS_SECTION_JOURNAL, |ui| {
        super::journal::show(ui, ctx.state)
    });
    ui.add_space(GAP);
    panel(ui, text::SETTINGS_SECTION_ABOUT, |ui| {
        let p = Palette::current(ui.ctx());
        ui.label(format!("Asiba {}", env!("CARGO_PKG_VERSION")));
        ui.label(RichText::new(text::ABOUT_LINE).color(p.text_secondary));
    });
    action
}

fn theme_picker(ui: &mut Ui, current: ThemeChoice) -> Option<Action> {
    let mut chosen = current;
    ui.horizontal(|ui| {
        ui.label(text::SETTINGS_THEME);
        chip_value(
            ui,
            &mut chosen,
            ThemeChoice::Dark,
            text::SETTINGS_THEME_DARK,
        );
        chip_value(
            ui,
            &mut chosen,
            ThemeChoice::Light,
            text::SETTINGS_THEME_LIGHT,
        );
    });
    (chosen != current).then_some(Action::SetTheme(chosen))
}

fn paths_grid(ui: &mut Ui, paths: &Paths, config: &AppConfig) {
    let p = Palette::current(ui.ctx());
    let rows = [
        (
            text::SETTINGS_CONFIG_DIR,
            paths.config_dir.display().to_string(),
        ),
        (
            text::SETTINGS_SERVERS_DIR,
            config.servers_dir(paths).display().to_string(),
        ),
        (
            text::SETTINGS_DATA_DIR,
            paths.data_dir.display().to_string(),
        ),
    ];
    Grid::new("settings-paths")
        .num_columns(2)
        .spacing([16.0, 4.0])
        .show(ui, |ui| {
            for (label, value) in rows {
                ui.label(RichText::new(label).color(p.text_secondary));
                ui.monospace(value);
                ui.end_row();
            }
        });
}
