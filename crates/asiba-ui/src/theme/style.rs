use egui::epaint::Shadow;
use egui::style::{Selection, WidgetVisuals, Widgets};
use egui::{Context, CornerRadius, FontFamily, FontId, Margin, Stroke, TextStyle, Vec2, Visuals};

use super::palette::Palette;

const RADIUS: u8 = 2;
const BORDER: f32 = 1.0;

pub fn apply(ctx: &Context, palette: &Palette) {
    let mut style = (*ctx.global_style()).clone();
    style.visuals = visuals(palette);
    style.spacing.item_spacing = Vec2::new(8.0, 6.0);
    style.spacing.button_padding = Vec2::new(10.0, 4.0);
    style.spacing.window_margin = Margin::same(8);
    style.spacing.menu_margin = Margin::same(6);
    style.spacing.interact_size = Vec2::new(40.0, 22.0);
    style.spacing.indent = 12.0;
    style.spacing.text_edit_width = 320.0;
    style.spacing.combo_width = 200.0;
    style.text_styles = text_styles();
    ctx.set_global_style(style);
}

fn text_styles() -> std::collections::BTreeMap<TextStyle, FontId> {
    [
        (
            TextStyle::Small,
            FontId::new(11.0, FontFamily::Proportional),
        ),
        (TextStyle::Body, FontId::new(13.0, FontFamily::Proportional)),
        (
            TextStyle::Button,
            FontId::new(13.0, FontFamily::Proportional),
        ),
        (
            TextStyle::Heading,
            FontId::new(17.0, super::fonts::semibold()),
        ),
        (
            TextStyle::Monospace,
            FontId::new(12.5, FontFamily::Monospace),
        ),
    ]
    .into_iter()
    .collect()
}

fn visuals(p: &Palette) -> Visuals {
    let mut visuals = if p.dark {
        Visuals::dark()
    } else {
        Visuals::light()
    };
    visuals.panel_fill = p.bg_window;
    visuals.window_fill = p.bg_raised;
    visuals.window_stroke = Stroke::new(BORDER, p.border_active);
    visuals.window_shadow = Shadow::NONE;
    visuals.popup_shadow = Shadow::NONE;
    visuals.window_corner_radius = CornerRadius::same(RADIUS);
    visuals.menu_corner_radius = CornerRadius::same(RADIUS);
    visuals.extreme_bg_color = p.bg_window;
    visuals.faint_bg_color = p.bg_raised;
    visuals.code_bg_color = p.bg_window;
    visuals.text_edit_bg_color = Some(p.bg_window);
    visuals.hyperlink_color = p.accent;
    visuals.warn_fg_color = p.warning;
    visuals.error_fg_color = p.critical;
    visuals.selection = Selection {
        bg_fill: p.accent_bg,
        stroke: Stroke::new(BORDER, p.accent),
    };
    visuals.widgets = widgets(p);
    visuals.striped = true;
    visuals.button_frame = true;
    visuals.collapsing_header_frame = false;
    visuals.indent_has_left_vline = false;
    visuals
}

fn widgets(p: &Palette) -> Widgets {
    let base = |bg: egui::Color32, border: egui::Color32, text: egui::Color32| WidgetVisuals {
        bg_fill: bg,
        weak_bg_fill: bg,
        bg_stroke: Stroke::new(BORDER, border),
        corner_radius: CornerRadius::same(RADIUS),
        fg_stroke: Stroke::new(BORDER, text),
        expansion: 0.0,
    };
    Widgets {
        noninteractive: base(p.bg_panel, p.border, p.text),
        inactive: base(p.bg_raised, p.border, p.text),
        hovered: base(p.bg_hover, p.border_active, p.text),
        active: base(p.accent_bg, p.accent, p.text),
        open: base(p.bg_hover, p.border_active, p.text),
    }
}
