use egui::{Context, Id, Ui};

use crate::text;

const SHOW_PASSED_KEY: &str = "security-show-passed";

pub fn is_showing_passed(ctx: &Context, server: &str) -> bool {
    ctx.data(|d| d.get_temp(Id::new((SHOW_PASSED_KEY, server))))
        .unwrap_or(false)
}

pub fn toggle(ui: &mut Ui, server: &str) -> bool {
    let id = Id::new((SHOW_PASSED_KEY, server));
    let mut value = is_showing_passed(ui.ctx(), server);
    ui.checkbox(&mut value, text::SEC_SHOW_PASSED);
    ui.ctx().data_mut(|d| d.insert_temp(id, value));
    value
}
