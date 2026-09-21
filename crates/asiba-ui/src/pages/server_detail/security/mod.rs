mod access;
mod attacks;
mod audit_list;
mod problem;
mod state;

use egui::{RichText, Ui};

use super::{DetailContext, audit as ai_audit, section_analysis};
use crate::components::{cached_audit, chip, panel, panel_plain};
use crate::modules::Tab;
use crate::pages::Action;
use crate::text;
use crate::theme::{GAP, Palette};

pub use state::{Subpage, store as select_subpage};

pub fn show(ui: &mut Ui, ctx: &DetailContext<'_>) -> Option<Action> {
    let server = &ctx.server.spec.id;
    let mut subpage = state::load(ui.ctx(), server);
    let analysis_action = section_analysis(ui, ctx, Tab::Security);
    let audit = cached_audit(ui.ctx(), ctx.server, ctx.state);
    let page_action = panel_plain(ui, |ui| {
        subpage_bar(ui, &mut subpage);
        ui.add_space(GAP);
        match subpage.clone() {
            Subpage::Audit => audit_list::show(ui, ctx, &audit, &mut subpage),
            Subpage::Access => access::show(ui, ctx),
            Subpage::Attacks => attacks::show(ui, ctx),
            Subpage::Problem { key, .. } => problem::show(ui, ctx, &audit, &key, &mut subpage),
        }
    });
    state::store(ui.ctx(), server, subpage);
    ui.add_space(GAP);
    let ai_action = panel(ui, text::AI_AUDIT_TITLE, |ui| ai_audit::show(ui, ctx));
    page_action.or(analysis_action).or(ai_action)
}

fn subpage_bar(ui: &mut Ui, subpage: &mut Subpage) {
    let p = Palette::current(ui.ctx());
    let is_problem = matches!(subpage, Subpage::Problem { .. });
    ui.horizontal_wrapped(|ui| {
        let items = [
            (Subpage::Audit, text::SEC_SUB_AUDIT),
            (Subpage::Access, text::SEC_SUB_ACCESS),
            (Subpage::Attacks, text::SEC_SUB_ATTACKS),
        ];
        for (target, label) in items {
            let selected = *subpage == target || (is_problem && target == Subpage::Audit);
            let color = if selected { p.text } else { p.text_secondary };
            if chip(ui, selected, RichText::new(label).color(color)).clicked() {
                *subpage = target;
            }
        }
    });
}
