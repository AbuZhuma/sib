use asiba_modules::security::{self, SecuritySnapshot};
use egui::{RichText, Ui};

use super::DetailContext;
use crate::modules::security_view::tables;
use crate::pages::Action;
use crate::text;
use crate::theme::{GAP, Palette};

pub fn show(ui: &mut Ui, ctx: &DetailContext<'_>) -> Option<Action> {
    let Some(snapshot) = ctx.server.data::<SecuritySnapshot>(security::ID) else {
        no_data(ui);
        return None;
    };
    tables::logins(ui, snapshot);
    ui.add_space(GAP);
    tables::sudo_calls(ui, snapshot);
    None
}

pub fn no_data(ui: &mut Ui) {
    let p = Palette::current(ui.ctx());
    ui.label(RichText::new(text::SEC_NO_DATA).color(p.text_muted));
}
