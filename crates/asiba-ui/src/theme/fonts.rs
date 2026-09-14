use std::sync::Arc;

use egui::{Context, FontData, FontDefinitions, FontFamily};

const INTER_REGULAR: &[u8] = include_bytes!("../../../../assets/fonts/Inter-Regular.ttf");
const INTER_MEDIUM: &[u8] = include_bytes!("../../../../assets/fonts/Inter-Medium.ttf");
const INTER_SEMIBOLD: &[u8] = include_bytes!("../../../../assets/fonts/Inter-SemiBold.ttf");
const MONO_REGULAR: &[u8] = include_bytes!("../../../../assets/fonts/JetBrainsMono-Regular.ttf");
const MONO_MEDIUM: &[u8] = include_bytes!("../../../../assets/fonts/JetBrainsMono-Medium.ttf");

pub const FAMILY_MEDIUM: &str = "inter-medium";
pub const FAMILY_SEMIBOLD: &str = "inter-semibold";
pub const FAMILY_MONO_MEDIUM: &str = "mono-medium";

pub fn install(ctx: &Context) {
    let mut fonts = FontDefinitions::default();
    let entries = [
        ("inter", INTER_REGULAR),
        (FAMILY_MEDIUM, INTER_MEDIUM),
        (FAMILY_SEMIBOLD, INTER_SEMIBOLD),
        ("mono", MONO_REGULAR),
        (FAMILY_MONO_MEDIUM, MONO_MEDIUM),
    ];
    for (name, bytes) in entries {
        fonts
            .font_data
            .insert(name.to_owned(), Arc::new(FontData::from_static(bytes)));
    }
    fonts
        .families
        .entry(FontFamily::Proportional)
        .or_default()
        .insert(0, "inter".to_owned());
    fonts
        .families
        .entry(FontFamily::Monospace)
        .or_default()
        .insert(0, "mono".to_owned());
    for name in [FAMILY_MEDIUM, FAMILY_SEMIBOLD, FAMILY_MONO_MEDIUM] {
        let fallback = vec![name.to_owned(), "inter".to_owned(), "mono".to_owned()];
        fonts
            .families
            .insert(FontFamily::Name(name.into()), fallback);
    }
    ctx.set_fonts(fonts);
}

pub fn semibold() -> FontFamily {
    FontFamily::Name(FAMILY_SEMIBOLD.into())
}
