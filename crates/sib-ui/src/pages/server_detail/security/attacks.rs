use egui::Ui;
use sib_modules::security::{self, SecuritySnapshot};

use super::DetailContext;
use super::access::no_data;
use crate::modules::security_view::tables;
use crate::pages::Action;
use crate::pages::server_detail::to_action;
use crate::theme::GAP;

pub fn show(ui: &mut Ui, ctx: &DetailContext<'_>) -> Option<Action> {
    let Some(snapshot) = ctx.server.data::<SecuritySnapshot>(security::ID) else {
        no_data(ui);
        return None;
    };
    let mut action = tables::attackers(ui, snapshot, &ctx.shared);
    ui.add_space(GAP);
    action = tables::bans(ui, snapshot).or(action);
    action.map(|view_action| to_action(view_action, ctx.server, security::ID))
}
