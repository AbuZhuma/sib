use asiba_config::SummaryLayout;
use asiba_core::{AppState, AuditScope, AuditTarget, IgnoredIncident, ServerState};
use egui::{Id, RichText, Ui};

use super::{connection, header, modules_table, open_for_incident, select_tab};
use crate::components::{
    AiBlock, IncidentLine, ai_block, has_report, incident_line, incident_scope, panel,
    panel_with_controls,
};
use crate::modules::{ModuleView, Tab, ViewShared};
use crate::pages::Action;
use crate::text;
use crate::theme::{GAP, Palette};

const EDIT_KEY: &str = "summary-layout-editing";

pub struct SummaryContext<'a> {
    pub server: &'a ServerState,
    pub state: &'a AppState,
    pub can_audit: bool,
    pub views: &'a [Box<dyn ModuleView>],
    pub layout: &'a SummaryLayout,
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
    let editing = edit_toggle(ui);
    let available = available_views(ctx);
    let mut layout = ctx.layout.clone();
    let grid = GridContext {
        ctx,
        available: &available,
        editing,
    };
    let mut changed = module_grid(ui, &grid, &mut layout);
    if editing {
        changed |= hidden_list(ui, &mut layout, &available);
    }
    if changed {
        action = Some(Action::SaveLayout {
            server: server.spec.id.clone(),
            layout,
        });
    }
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

struct GridContext<'a> {
    ctx: &'a SummaryContext<'a>,
    available: &'a [&'a dyn ModuleView],
    editing: bool,
}

fn available_views<'a>(ctx: &'a SummaryContext<'a>) -> Vec<&'a dyn ModuleView> {
    ctx.views
        .iter()
        .map(Box::as_ref)
        .filter(|v| v.has_content(ctx.server))
        .collect()
}

fn visible_views<'a>(
    available: &[&'a dyn ModuleView],
    layout: &SummaryLayout,
    ids: &[&str],
) -> Vec<&'a dyn ModuleView> {
    layout
        .arrange(ids)
        .into_iter()
        .filter(|id| !layout.is_hidden(id))
        .filter_map(|id| available.iter().copied().find(|v| v.id().0 == id))
        .collect()
}

fn module_grid(ui: &mut Ui, grid: &GridContext<'_>, layout: &mut SummaryLayout) -> bool {
    let server = grid.ctx.server;
    let ids: Vec<&str> = grid.available.iter().map(|v| v.id().0).collect();
    let visible = visible_views(grid.available, layout, &ids);
    let mut changed = false;
    ui.columns(2, |columns| {
        for (index, view) in visible.iter().enumerate() {
            let column = &mut columns[index % 2];
            let id = view.id().0;
            panel_with_controls(
                column,
                view.title(),
                |ui| {
                    if grid.editing {
                        changed |= layout_controls(ui, layout, &ids, id);
                    } else {
                        open_button(ui, view.tab());
                    }
                },
                |ui| view.summary(ui, server, grid.ctx.shared),
            );
            column.add_space(GAP);
        }
        fixed_panels(columns, server);
    });
    changed
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

fn edit_toggle(ui: &mut Ui) -> bool {
    let id = Id::new(EDIT_KEY);
    let mut editing: bool = ui.ctx().data(|d| d.get_temp(id)).unwrap_or(false);
    ui.horizontal(|ui| {
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            let label = if editing {
                text::DETAIL_LAYOUT_DONE
            } else {
                text::DETAIL_LAYOUT
            };
            if ui.small_button(label).clicked() {
                editing = !editing;
            }
        });
    });
    ui.ctx().data_mut(|d| d.insert_temp(id, editing));
    editing
}

fn layout_controls(ui: &mut Ui, layout: &mut SummaryLayout, ids: &[&str], id: &str) -> bool {
    let mut changed = false;
    if ui.small_button(text::DETAIL_HIDE).clicked() {
        layout.toggle_hidden(id);
        changed = true;
    }
    if ui.small_button(text::DETAIL_DOWN).clicked() {
        layout.shift(ids, id, 1);
        changed = true;
    }
    if ui.small_button(text::DETAIL_UP).clicked() {
        layout.shift(ids, id, -1);
        changed = true;
    }
    changed
}

fn hidden_list(ui: &mut Ui, layout: &mut SummaryLayout, views: &[&dyn ModuleView]) -> bool {
    let p = Palette::current(ui.ctx());
    let hidden: Vec<&dyn ModuleView> = views
        .iter()
        .copied()
        .filter(|v| layout.is_hidden(v.id().0))
        .collect();
    if hidden.is_empty() {
        return false;
    }
    let mut changed = false;
    ui.horizontal_wrapped(|ui| {
        ui.label(RichText::new(text::DETAIL_HIDDEN).color(p.text_secondary));
        for view in hidden {
            let label = format!("{} {}", text::DETAIL_SHOW, view.title());
            if ui.small_button(label).clicked() {
                layout.toggle_hidden(view.id().0);
                changed = true;
            }
        }
    });
    ui.add_space(GAP);
    changed
}
