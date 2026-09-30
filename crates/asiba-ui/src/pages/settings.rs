use asiba_config::{AppConfig, Paths, ThemeChoice};
use asiba_core::AppState;
use egui::{Grid, Id, RichText, Ui};

use super::Action;
use crate::components::{chip, chip_value, help, page_title, panel, scroll, section_label, toggle};
use crate::text;
use crate::theme::{GAP, Palette};

const SECTION_KEY: &str = "settings-section";

#[derive(Clone, Copy, PartialEq, Eq)]
enum Section {
    General,
    Collection,
    Ai,
    Alerts,
    Journal,
    About,
}

impl Section {
    const ALL: [Self; 6] = [
        Self::General,
        Self::Collection,
        Self::Ai,
        Self::Alerts,
        Self::Journal,
        Self::About,
    ];

    fn label(self) -> &'static str {
        match self {
            Self::General => text::SETTINGS_SECTION_GENERAL,
            Self::Collection => text::SETTINGS_SECTION_COLLECTION,
            Self::Ai => text::SETTINGS_SECTION_AI,
            Self::Alerts => text::SETTINGS_SECTION_ALERTS,
            Self::Journal => text::SETTINGS_SECTION_JOURNAL,
            Self::About => text::SETTINGS_SECTION_ABOUT,
        }
    }
}

pub struct SettingsContext<'a> {
    pub paths: &'a Paths,
    pub config: &'a AppConfig,
    pub state: &'a AppState,
}

pub fn show(ui: &mut Ui, ctx: &SettingsContext<'_>) -> Option<Action> {
    page_title(ui, text::SETTINGS_TITLE);
    let id = Id::new(SECTION_KEY);
    let mut section: Section = ui
        .ctx()
        .data(|d| d.get_temp(id))
        .unwrap_or(Section::General);
    if nav(ui, &mut section) {
        ui.ctx().data_mut(|d| d.insert_temp(id, section));
    }
    ui.add_space(GAP);
    let mut action = None;
    scroll::vertical().show(ui, |ui| {
        action = panel(ui, section.label(), |ui| body(ui, ctx, section));
    });
    action
}

fn nav(ui: &mut Ui, current: &mut Section) -> bool {
    let p = Palette::current(ui.ctx());
    let mut changed = false;
    ui.horizontal_wrapped(|ui| {
        for section in Section::ALL {
            let selected = section == *current;
            let color = if selected { p.text } else { p.text_secondary };
            if chip(ui, selected, RichText::new(section.label()).color(color)).clicked() {
                *current = section;
                changed = true;
            }
        }
    });
    changed
}

fn body(ui: &mut Ui, ctx: &SettingsContext<'_>, section: Section) -> Option<Action> {
    let config = ctx.config;
    match section {
        Section::General => general(ui, config),
        Section::Collection => super::collection_settings::show(ui, config),
        Section::Ai => super::ai_settings::show(ui, config, ctx.state),
        Section::Alerts => super::alert_rules::show(ui, config),
        Section::Journal => {
            super::journal::show(ui, ctx.state);
            None
        }
        Section::About => {
            about(ui, ctx.paths, config);
            None
        }
    }
}

fn general(ui: &mut Ui, config: &AppConfig) -> Option<Action> {
    let theme = theme_picker(ui, config.theme);
    ui.add_space(GAP);
    section_label(ui, text::SETTINGS_SECTION_PRIVACY);
    geolocation_toggle(ui, config.geolocation).or(theme)
}

fn about(ui: &mut Ui, paths: &Paths, config: &AppConfig) {
    let p = Palette::current(ui.ctx());
    ui.label(format!("Asiba {}", env!("CARGO_PKG_VERSION")));
    ui.label(RichText::new(text::ABOUT_LINE).color(p.text_secondary));
    ui.add_space(GAP);
    paths_grid(ui, paths, config);
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
    let mut enabled = current;
    ui.horizontal(|ui| {
        toggle(ui, &mut enabled, text::SETTINGS_GEOLOCATION);
        help(ui, text::SETTINGS_GEOLOCATION_HINT);
    });
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
