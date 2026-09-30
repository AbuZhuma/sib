mod fonts;
mod palette;
mod style;

pub use palette::Palette;

use sib_config::ThemeChoice;

pub const GAP: f32 = 8.0;
pub const GAP_SMALL: f32 = 4.0;
pub const ROW_HEIGHT: f32 = 24.0;
pub const SIDEBAR_WIDTH: f32 = 200.0;
pub const STATUSBAR_HEIGHT: f32 = 28.0;
pub const CARD_WIDTH: f32 = 300.0;
pub const FIELD_WIDTH: f32 = 320.0;
pub const SECONDARY_PLOT_HEIGHT: f32 = 130.0;
pub const SUMMARY_SPARKLINE_HEIGHT: f32 = 64.0;
pub const SUMMARY_RATE_HEIGHT: f32 = 44.0;
pub const MINI_MAP_HEIGHT: f32 = 260.0;
pub const LONG_TABLE_FRACTION: f32 = 0.8;

pub fn install(ctx: &egui::Context, choice: ThemeChoice) {
    fonts::install(ctx);
    apply(ctx, choice);
}

pub fn apply(ctx: &egui::Context, choice: ThemeChoice) {
    let palette = Palette::for_choice(choice);
    style::apply(ctx, &palette);
    palette.store(ctx);
}
