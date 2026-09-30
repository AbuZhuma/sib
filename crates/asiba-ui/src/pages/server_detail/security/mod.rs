mod access;
mod attacks;
mod audit_list;
mod checks;
mod problem;
mod state;

use asiba_core::ServerState;
use asiba_modules::security::{self, SecuritySnapshot};
use egui::{RichText, Ui};

use super::{DetailContext, audit as ai_audit, section_analysis};
use crate::components::block_settings::{self, Popup};
use crate::components::{cached_audit, chip, panel, panel_plain};
use crate::modules::Tab;
use crate::pages::Action;
use crate::text;
use crate::theme::{GAP, Palette};

pub use state::{Subpage, store as select_subpage};

pub fn show(ui: &mut Ui, ctx: &DetailContext<'_>) -> Option<Action> {
    let server = &ctx.server.spec.id;
    let subpages = available_subpages(ctx.server);
    let mut subpage = state::load(ui.ctx(), server);
    if !is_available(&subpages, &subpage) {
        subpage = Subpage::Audit;
    }
    let analysis_action = section_analysis(ui, ctx, Tab::Security);
    let audit = cached_audit(ui.ctx(), ctx.server, ctx.state);
    let page_action = panel_plain(ui, |ui| {
        subpage_bar(ui, &mut subpage, &subpages);
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
    let checks_action = check_settings(ui, ctx);
    let ai_action = panel(ui, text::AI_AUDIT_TITLE, |ui| ai_audit::show(ui, ctx));
    page_action
        .or(checks_action)
        .or(analysis_action)
        .or(ai_action)
}

const CHECKS_KEY: &str = "audit-checks";

fn check_settings(ui: &mut Ui, ctx: &DetailContext<'_>) -> Option<Action> {
    let spec = Popup {
        key: CHECKS_KEY,
        title: text::CHECKS_TITLE,
    };
    let mut draft = checks::load(ui.ctx(), ctx.server);
    let action = block_settings::popup(ui, &spec, |ui| {
        block_settings::scrolled(ui, |ui| checks::body(ui, &mut draft, ctx.server, ctx.state));
        checks::controls(ui, &mut draft, ctx.server)
    })?;
    checks::store(ui.ctx(), ctx.server, draft, action.is_some());
    action
}

fn available_subpages(server: &ServerState) -> Vec<(Subpage, &'static str)> {
    let snapshot = server.data::<SecuritySnapshot>(security::ID);
    let has_rows = |rows: fn(&SecuritySnapshot) -> bool| snapshot.is_some_and(rows);
    let mut items = vec![(Subpage::Audit, text::SEC_SUB_AUDIT)];
    if has_rows(|s| !s.logins.is_empty() || !s.sudo_calls.is_empty()) {
        items.push((Subpage::Access, text::SEC_SUB_ACCESS));
    }
    if has_rows(|s| !s.attackers.is_empty() || !s.bans.is_empty()) {
        items.push((Subpage::Attacks, text::SEC_SUB_ATTACKS));
    }
    items
}

fn is_available(subpages: &[(Subpage, &'static str)], subpage: &Subpage) -> bool {
    matches!(subpage, Subpage::Audit | Subpage::Problem { .. })
        || subpages.iter().any(|(target, _)| target == subpage)
}

fn subpage_bar(ui: &mut Ui, subpage: &mut Subpage, items: &[(Subpage, &'static str)]) {
    let p = Palette::current(ui.ctx());
    let is_problem = matches!(subpage, Subpage::Problem { .. });
    ui.horizontal_wrapped(|ui| {
        for (target, label) in items.iter().cloned() {
            let selected = *subpage == target || (is_problem && target == Subpage::Audit);
            let color = if selected { p.text } else { p.text_secondary };
            if chip(ui, selected, RichText::new(label).color(color)).clicked() {
                *subpage = target;
            }
        }
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            block_settings::gear(ui, CHECKS_KEY);
        });
    });
}
