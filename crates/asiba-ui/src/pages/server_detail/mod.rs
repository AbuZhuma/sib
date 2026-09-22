pub mod audit;
mod connection;
mod events;
pub mod files;
mod header;
mod modules_table;
pub mod security;
mod summary;

use asiba_config::{AiConfig, SummaryLayout};
use asiba_core::{
    AppState, AuditScope, AuditTarget, Incident, IncidentKind, ModuleId, ServerState,
};
use asiba_modules::files as files_module;
use egui::{Id, RichText, Ui};

use super::Action;
use crate::components::{AiBlock, ai_block, chip, incident_tab, panel, panel_plain, scroll};
use crate::modules::{ModuleView, Tab, ViewAction, ViewShared};
use crate::pages::inspector::{self, Inspector};
use crate::text;
use crate::theme::{GAP, Palette};

const TAB_KEY: &str = "server-detail-tab";

pub struct DetailContext<'a> {
    pub server: &'a ServerState,
    pub state: &'a AppState,
    pub ai: &'a AiConfig,
    pub views: &'a [Box<dyn ModuleView>],
    pub inspector: Option<&'a Inspector>,
    pub files: Option<&'a files::FileBrowser>,
    pub layout: &'a SummaryLayout,
    pub shared: ViewShared<'a>,
}

pub fn select_tab(ctx: &egui::Context, tab: Tab) {
    ctx.data_mut(|d| d.insert_temp(Id::new(TAB_KEY), tab));
}

pub fn open_for_incident(ctx: &egui::Context, state: &AppState, incident: &Incident) {
    select_tab(ctx, incident_tab(state, incident));
    if incident.kind == IncidentKind::SecurityCheck {
        security::select_subpage(
            ctx,
            &incident.server,
            security::Subpage::problem(&incident.subject),
        );
    }
}

pub fn show(ui: &mut Ui, ctx: &DetailContext<'_>) -> Option<Action> {
    let (server, views) = (ctx.server, ctx.views);
    let mut action = header::show(ui, server);
    let tabs = visible_tabs(server, views);
    let mut tab = current_tab(ui, &tabs);
    if tab_bar(ui, &tabs, &mut tab) {
        select_tab(ui.ctx(), tab);
    }
    if let Some(inspector) = ctx
        .inspector
        .filter(|i| i.is_shown_on(&server.spec.id, tab))
    {
        panel(ui, text::INSPECTOR_TITLE, |ui| {
            if let Some(next) = inspector::show(ui, inspector) {
                action = Some(next);
            }
        });
        ui.add_space(GAP);
    }
    scroll::vertical().show(ui, |ui| {
        if let Some(next) = tab_content(ui, ctx, tab) {
            action = Some(next);
        }
        if matches!(tab, Tab::Summary | Tab::Logs) {
            events::show(ui, server);
        }
    });
    action
}

fn tab_content(ui: &mut Ui, ctx: &DetailContext<'_>, tab: Tab) -> Option<Action> {
    match tab {
        Tab::Summary => summary::show(
            ui,
            &summary::SummaryContext {
                server: ctx.server,
                state: ctx.state,
                can_audit: ctx.ai.is_ready(),
                views: ctx.views,
                layout: ctx.layout,
                shared: &ctx.shared,
            },
        ),
        Tab::Files => files::show(ui, ctx),
        Tab::Security => security::show(ui, ctx),
        other => {
            let analysis_action = section_analysis(ui, ctx, other);
            pages_for(ui, ctx, other).or(analysis_action)
        }
    }
}

fn visible_tabs(server: &ServerState, views: &[Box<dyn ModuleView>]) -> Vec<Tab> {
    Tab::ALL
        .into_iter()
        .filter(|tab| {
            matches!(tab, Tab::Summary | Tab::Security)
                || (*tab == Tab::Files && has_files_module(server))
                || views
                    .iter()
                    .any(|v| v.tab() == *tab && v.has_content(server))
        })
        .collect()
}

fn has_files_module(server: &ServerState) -> bool {
    server
        .modules
        .get(&files_module::ID)
        .is_some_and(|module| module.availability.is_usable())
}

fn current_tab(ui: &Ui, tabs: &[Tab]) -> Tab {
    let stored: Option<Tab> = ui.ctx().data(|d| d.get_temp(Id::new(TAB_KEY)));
    stored.filter(|t| tabs.contains(t)).unwrap_or(Tab::Summary)
}

fn tab_bar(ui: &mut Ui, tabs: &[Tab], current: &mut Tab) -> bool {
    let p = Palette::current(ui.ctx());
    let mut changed = false;
    ui.horizontal_wrapped(|ui| {
        for tab in tabs {
            let selected = tab == current;
            let color = if selected { p.text } else { p.text_secondary };
            if chip(ui, selected, RichText::new(tab.label()).color(color)).clicked() {
                *current = *tab;
                changed = true;
            }
        }
    });
    ui.add_space(GAP);
    changed
}

fn section_analysis(ui: &mut Ui, ctx: &DetailContext<'_>, tab: Tab) -> Option<Action> {
    if !ctx.ai.consent {
        return None;
    }
    let key = tab.section_key()?;
    let block = AiBlock {
        state: ctx.state,
        target: AuditTarget::Server(ctx.server.spec.id.clone()),
        scope: AuditScope::Section {
            key: key.to_owned(),
        },
        title: text::AI_BLOCK_SECTION,
        can_audit: ctx.ai.is_ready(),
        auto_request: ctx.ai.is_section_auto(key),
    };
    let action = ai_block(ui, &block);
    ui.add_space(GAP);
    action
}

fn pages_for(ui: &mut Ui, ctx: &DetailContext<'_>, tab: Tab) -> Option<Action> {
    let (server, views) = (ctx.server, ctx.views);
    let mut action = None;
    let shown: Vec<&Box<dyn ModuleView>> = views
        .iter()
        .filter(|v| v.tab() == tab && v.has_content(server))
        .collect();
    let is_single = shown.len() == 1;
    for view in shown {
        let view_action = if is_single {
            panel_plain(ui, |ui| view.page(ui, server, &ctx.shared))
        } else {
            panel(ui, view.title(), |ui| view.page(ui, server, &ctx.shared))
        };
        if let Some(view_action) = view_action {
            action = Some(to_action(view_action, server, view.id()));
        }
        ui.add_space(GAP);
    }
    action
}

fn to_action(view_action: ViewAction, server: &ServerState, module: ModuleId) -> Action {
    let server = server.spec.id.clone();
    match view_action {
        ViewAction::Query(request) => Action::Query {
            server,
            module,
            request,
        },
        ViewAction::Backfill => Action::Backfill { server, module },
        ViewAction::Act { spec, request } => Action::AskPerform {
            server,
            spec,
            request,
        },
    }
}
