use asiba_config::ThemeChoice;
use egui::{Color32, Context, Id};

const STORE_KEY: &str = "asiba.palette";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Palette {
    pub dark: bool,
    pub bg_window: Color32,
    pub bg_panel: Color32,
    pub bg_raised: Color32,
    pub bg_hover: Color32,
    pub border: Color32,
    pub border_active: Color32,
    pub text: Color32,
    pub text_secondary: Color32,
    pub text_muted: Color32,
    pub accent: Color32,
    pub accent_bg: Color32,
    pub ok: Color32,
    pub warning: Color32,
    pub critical: Color32,
    pub info: Color32,
    pub offline: Color32,
    pub chart: [Color32; 6],
}

impl Palette {
    pub fn for_choice(choice: ThemeChoice) -> Self {
        match choice {
            ThemeChoice::Dark => Self::dark(),
            ThemeChoice::Light => Self::light(),
        }
    }

    pub fn dark() -> Self {
        Self {
            dark: true,
            bg_window: Color32::from_rgb(0x1c, 0x1e, 0x21),
            bg_panel: Color32::from_rgb(0x24, 0x27, 0x2b),
            bg_raised: Color32::from_rgb(0x2c, 0x30, 0x35),
            bg_hover: Color32::from_rgb(0x34, 0x39, 0x3f),
            border: Color32::from_rgb(0x3a, 0x3f, 0x45),
            border_active: Color32::from_rgb(0x5a, 0x61, 0x6a),
            text: Color32::from_rgb(0xd7, 0xda, 0xdf),
            text_secondary: Color32::from_rgb(0x8b, 0x92, 0x9a),
            text_muted: Color32::from_rgb(0x5f, 0x66, 0x6e),
            accent: Color32::from_rgb(0x4f, 0x8c, 0xc9),
            accent_bg: Color32::from_rgb(0x27, 0x3a, 0x4e),
            ok: Color32::from_rgb(0x3f, 0x9d, 0x5a),
            warning: Color32::from_rgb(0xd0, 0xa1, 0x3a),
            critical: Color32::from_rgb(0xc9, 0x4f, 0x4f),
            info: Color32::from_rgb(0x5a, 0x8f, 0xd0),
            offline: Color32::from_rgb(0x6b, 0x70, 0x76),
            chart: [
                Color32::from_rgb(0x6f, 0xa3, 0xd8),
                Color32::from_rgb(0x8f, 0xb8, 0x9a),
                Color32::from_rgb(0xd4, 0xb2, 0x6a),
                Color32::from_rgb(0xc9, 0x8a, 0x8a),
                Color32::from_rgb(0xa8, 0x9c, 0xd4),
                Color32::from_rgb(0x9a, 0xa4, 0xae),
            ],
        }
    }

    pub fn light() -> Self {
        Self {
            dark: false,
            bg_window: Color32::from_rgb(0xe9, 0xea, 0xec),
            bg_panel: Color32::from_rgb(0xf4, 0xf5, 0xf6),
            bg_raised: Color32::from_rgb(0xff, 0xff, 0xff),
            bg_hover: Color32::from_rgb(0xe3, 0xe6, 0xe9),
            border: Color32::from_rgb(0xc9, 0xcc, 0xd1),
            border_active: Color32::from_rgb(0x9a, 0xa0, 0xa8),
            text: Color32::from_rgb(0x1f, 0x22, 0x26),
            text_secondary: Color32::from_rgb(0x5b, 0x61, 0x68),
            text_muted: Color32::from_rgb(0x8b, 0x91, 0x98),
            accent: Color32::from_rgb(0x2f, 0x6f, 0xae),
            accent_bg: Color32::from_rgb(0xd6, 0xe4, 0xf3),
            ok: Color32::from_rgb(0x2e, 0x8b, 0x4b),
            warning: Color32::from_rgb(0xb8, 0x89, 0x1f),
            critical: Color32::from_rgb(0xb8, 0x3e, 0x3e),
            info: Color32::from_rgb(0x3d, 0x78, 0xb8),
            offline: Color32::from_rgb(0x8a, 0x90, 0x99),
            chart: [
                Color32::from_rgb(0x3b, 0x78, 0xb4),
                Color32::from_rgb(0x4d, 0x8c, 0x62),
                Color32::from_rgb(0xa8, 0x82, 0x2c),
                Color32::from_rgb(0xb0, 0x4e, 0x4e),
                Color32::from_rgb(0x6f, 0x5f, 0xb0),
                Color32::from_rgb(0x6b, 0x74, 0x7e),
            ],
        }
    }

    pub fn store(self, ctx: &Context) {
        ctx.data_mut(|data| data.insert_temp(Id::new(STORE_KEY), self));
    }

    pub fn current(ctx: &Context) -> Self {
        ctx.data(|data| data.get_temp(Id::new(STORE_KEY)))
            .unwrap_or_else(Self::dark)
    }
}
