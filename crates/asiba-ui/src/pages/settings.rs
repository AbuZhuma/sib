use asiba_config::{AppConfig, Paths, ThemeChoice};
use egui::{Grid, RichText, Ui};

use super::Action;
use crate::components::{page_title, panel};
use crate::text;
use crate::theme::{GAP, Palette};

pub fn show(ui: &mut Ui, paths: &Paths, config: &AppConfig) -> Option<Action> {
    page_title(ui, text::SETTINGS_TITLE);
    let mut action = None;
    panel(ui, text::SETTINGS_SECTION_APPEARANCE, |ui| {
        action = theme_picker(ui, config.theme);
    });
    ui.add_space(GAP);
    panel(ui, text::SETTINGS_SECTION_PATHS, |ui| {
        paths_grid(ui, paths, config)
    });
    action
}

fn theme_picker(ui: &mut Ui, current: ThemeChoice) -> Option<Action> {
    let mut chosen = current;
    ui.horizontal(|ui| {
        ui.label(text::SETTINGS_THEME);
        ui.selectable_value(&mut chosen, ThemeChoice::Dark, text::SETTINGS_THEME_DARK);
        ui.selectable_value(&mut chosen, ThemeChoice::Light, text::SETTINGS_THEME_LIGHT);
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
