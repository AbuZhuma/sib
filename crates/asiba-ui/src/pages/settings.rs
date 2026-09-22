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
    let mut section =
        |ui: &mut Ui, title: &str, body: &mut dyn FnMut(&mut Ui) -> Option<Action>| {
            if let Some(next) = panel(ui, title, |ui| body(ui)) {
                action = Some(next);
            }
            ui.add_space(GAP);
        };
    section(ui, text::SETTINGS_SECTION_APPEARANCE, &mut |ui| {
        theme_picker(ui, config.theme)
    });
    section(ui, text::SETTINGS_SECTION_PRIVACY, &mut |ui| {
        geolocation_toggle(ui, config.geolocation)
    });
    section(ui, text::SETTINGS_SECTION_PATHS, &mut |ui| {
        paths_grid(ui, paths, config);
        None
    });
    section(ui, text::SETTINGS_SECTION_COLLECTION, &mut |ui| {
        super::collection_settings::show(ui, config)
    });
    section(ui, text::SETTINGS_SECTION_AI, &mut |ui| {
        super::ai_settings::show(ui, config, ctx.state)
    });
    section(ui, text::SETTINGS_SECTION_ALERTS, &mut |ui| {
        super::alert_rules::show(ui, config)
    });
    section(ui, text::SETTINGS_SECTION_JOURNAL, &mut |ui| {
        super::journal::show(ui, ctx.state);
        None
    });
    section(ui, text::SETTINGS_SECTION_ABOUT, &mut |ui| {
        about(ui);
        None
    });
    action
}

fn about(ui: &mut Ui) {
    let p = Palette::current(ui.ctx());
    ui.label(format!("Asiba {}", env!("CARGO_PKG_VERSION")));
    ui.label(RichText::new(text::ABOUT_LINE).color(p.text_secondary));
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

fn geolocation_toggle(ui: &mut Ui, current: bool) -> Option<Action> {
    let p = Palette::current(ui.ctx());
    let mut enabled = current;
    ui.checkbox(&mut enabled, text::SETTINGS_GEOLOCATION);
    ui.label(RichText::new(text::SETTINGS_GEOLOCATION_HINT).color(p.text_secondary));
    (enabled != current).then_some(Action::SetGeolocation(enabled))
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
