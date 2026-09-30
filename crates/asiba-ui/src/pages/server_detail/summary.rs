use asiba_core::{AppState, AuditScope, AuditTarget, IgnoredIncident, ServerState};
use egui::Ui;

use super::{connection, header, modules_table, open_for_incident, select_tab};
use crate::components::{
    AiBlock, IncidentLine, ai_block, has_report, incident_line, incident_scope, panel,
    panel_with_controls,
};
use crate::modules::{ModuleView, Tab, ViewShared};
use crate::pages::Action;
use crate::text;
use crate::theme::GAP;

pub struct SummaryContext<'a> {
    pub server: &'a ServerState,
    pub state: &'a AppState,
    pub can_audit: bool,
    pub views: &'a [Box<dyn ModuleView>],
    pub shared: &'a ViewShared<'a>,
}

pub fn show(ui: &mut Ui, ctx: &SummaryContext<'_>) -> Option<Action> {
    let server = ctx.server;
    let mut action = panel(ui, text::DETAIL_SECTION_CONNECTION, |ui| {
        connection::show(ui, server)
    });
    ui.add_space(GAP);
    if let Some(next) = incidents(ui, ctx) {
        action = Some(next);
    }
    if let Some(next) = full_audit(ui, ctx) {
        action = Some(next);
    }
    module_grid(ui, ctx);
    action
}

fn full_audit(ui: &mut Ui, ctx: &SummaryContext<'_>) -> Option<Action> {
    let target = AuditTarget::Server(ctx.server.spec.id.clone());
    let has_report = ctx.state.latest_audit(&target, &AuditScope::Full).is_some();
    if !ctx.can_audit && !has_report {
        return None;
    }
    let block = AiBlock {
        state: ctx.state,
        target,
        scope: AuditScope::Full,
        title: text::AI_BLOCK_TITLE,
        can_audit: ctx.can_audit,
        auto_request: false,
    };
    let action = ai_block(ui, &block);
    ui.add_space(GAP);
    action
}

fn available_views<'a>(ctx: &'a SummaryContext<'a>) -> Vec<&'a dyn ModuleView> {
    ctx.views
        .iter()
        .map(Box::as_ref)
        .filter(|v| v.has_content(ctx.server))
        .collect()
}

fn module_grid(ui: &mut Ui, ctx: &SummaryContext<'_>) {
    let server = ctx.server;
    let views = available_views(ctx);
    ui.columns(2, |columns| {
        for (index, view) in views.iter().enumerate() {
            let column = &mut columns[index % 2];
            panel_with_controls(
                column,
                view.title(),
                |ui| open_button(ui, view.tab()),
                |ui| view.summary(ui, server, ctx.shared),
            );
            column.add_space(GAP);
        }
        fixed_panels(columns, server);
    });
}

fn fixed_panels(columns: &mut [Ui], server: &ServerState) {
    let short = shorter_column(columns);
    panel(&mut columns[short], text::DETAIL_SECTION_MODULES, |ui| {
        modules_table::show(ui, server)
    });
    columns[short].add_space(GAP);
    let short = shorter_column(columns);
    panel(
        &mut columns[short],
        text::DETAIL_SECTION_DESCRIPTION,
        |ui| header::description(ui, server),
    );
}

fn incidents(ui: &mut Ui, ctx: &SummaryContext<'_>) -> Option<Action> {
    let mut incidents: Vec<&asiba_core::Incident> = ctx
        .state
        .active_incidents()
        .filter(|i| i.server == ctx.server.spec.id)
        .collect();
    if incidents.is_empty() {
        return None;
    }
    incidents.sort_by_key(|i| std::cmp::Reverse((i.severity, i.started_at)));
    let mut action = None;
    panel(ui, text::SECTION_INCIDENTS, |ui| {
        for incident in incidents {
            let line = IncidentLine {
                incident,
                show_server: false,
                can_audit: ctx.can_audit,
                has_report: has_report(ctx.state, incident),
            };
            let click = incident_line(ui, &line);
            if click.ignore {
                action = Some(Action::IgnoreIncident(IgnoredIncident::of(incident)));
            } else if click.audit {
                action = Some(Action::Audit {
                    target: AuditTarget::Server(incident.server.clone()),
                    scope: incident_scope(incident),
                });
            } else if click.open_report {
                select_tab(ui.ctx(), Tab::Security);
            } else if click.open_details {
                open_for_incident(ui.ctx(), ctx.state, incident);
            }
        }
    });
    ui.add_space(GAP);
    action
}

fn shorter_column(columns: &[Ui]) -> usize {
    let left = columns.first().map(|c| c.cursor().top()).unwrap_or(0.0);
    let right = columns.get(1).map(|c| c.cursor().top()).unwrap_or(0.0);
    usize::from(right < left)
}

fn open_button(ui: &mut Ui, tab: Tab) {
    if ui.small_button(text::DETAIL_OPEN).clicked() {
        select_tab(ui.ctx(), tab);
    }
}
