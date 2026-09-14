mod connection;
mod header;
mod modules_table;

use asiba_core::ServerState;
use egui::{Id, RichText, ScrollArea, Ui};

use super::Action;
use crate::components::panel;
use crate::modules::{ModuleView, Tab, ViewAction, has_data};
use crate::pages::inspector::{self, Inspector};
use crate::text;
use crate::theme::{GAP, Palette};

const TAB_KEY: &str = "server-detail-tab";

pub struct DetailContext<'a> {
    pub server: &'a ServerState,
    pub views: &'a [Box<dyn ModuleView>],
    pub inspector: Option<&'a Inspector>,
}

pub fn select_tab(ctx: &egui::Context, tab: Tab) {
    ctx.data_mut(|d| d.insert_temp(Id::new(TAB_KEY), tab));
}

pub fn show(ui: &mut Ui, ctx: &DetailContext<'_>) -> Option<Action> {
    let (server, views) = (ctx.server, ctx.views);
    let mut action = header::show(ui, server);
    let tabs = visible_tabs(server, views);
    let mut tab = current_tab(ui, &tabs);
    if tab_bar(ui, &tabs, &mut tab) {
        select_tab(ui.ctx(), tab);
    }
    if let Some(inspector) = ctx.inspector.filter(|i| i.server == server.spec.id) {
        panel(ui, text::INSPECTOR_TITLE, |ui| {
            if let Some(next) = inspector::show(ui, inspector) {
                action = Some(next);
            }
        });
        ui.add_space(GAP);
    }
    ScrollArea::vertical().show(ui, |ui| {
        let next = match tab {
            Tab::Summary => summary(ui, server, views),
            other => pages_for(ui, server, views, other),
        };
        if next.is_some() {
            action = next;
        }
    });
    action
}

fn visible_tabs(server: &ServerState, views: &[Box<dyn ModuleView>]) -> Vec<Tab> {
    Tab::ALL
        .into_iter()
        .filter(|tab| {
            *tab == Tab::Summary
                || views
                    .iter()
                    .any(|v| v.tab() == *tab && has_data(server, v.id()))
        })
        .collect()
}

fn current_tab(ui: &Ui, tabs: &[Tab]) -> Tab {
    let stored: Option<Tab> = ui.ctx().data(|d| d.get_temp(Id::new(TAB_KEY)));
    stored.filter(|t| tabs.contains(t)).unwrap_or(Tab::Summary)
}

fn tab_bar(ui: &mut Ui, tabs: &[Tab], current: &mut Tab) -> bool {
    let p = Palette::current(ui.ctx());
    let mut changed = false;
    ui.horizontal(|ui| {
        for tab in tabs {
            let selected = tab == current;
            let color = if selected { p.text } else { p.text_secondary };
            if ui
                .selectable_label(selected, RichText::new(tab.label()).color(color))
                .clicked()
            {
                *current = *tab;
                changed = true;
            }
        }
    });
    ui.add_space(GAP);
    changed
}

fn summary(ui: &mut Ui, server: &ServerState, views: &[Box<dyn ModuleView>]) -> Option<Action> {
    let action = panel(ui, text::DETAIL_SECTION_CONNECTION, |ui| {
        connection::show(ui, server)
    });
    ui.add_space(GAP);
    let with_data: Vec<&Box<dyn ModuleView>> =
        views.iter().filter(|v| has_data(server, v.id())).collect();
    ui.columns(2, |columns| {
        for (index, view) in with_data.iter().enumerate() {
            let column = &mut columns[index % 2];
            panel(column, view.title(), |ui| view.summary(ui, server));
            column.add_space(GAP);
        }
    });
    panel(ui, text::DETAIL_SECTION_MODULES, |ui| {
        modules_table::show(ui, server)
    });
    ui.add_space(GAP);
    panel(ui, text::DETAIL_SECTION_DESCRIPTION, |ui| {
        header::description(ui, server)
    });
    action
}

fn pages_for(
    ui: &mut Ui,
    server: &ServerState,
    views: &[Box<dyn ModuleView>],
    tab: Tab,
) -> Option<Action> {
    let mut action = None;
    for view in views
        .iter()
        .filter(|v| v.tab() == tab && has_data(server, v.id()))
    {
        let view_action = panel(ui, view.title(), |ui| view.page(ui, server));
        if let Some(ViewAction::Query(request)) = view_action {
            let server = server.spec.id.clone();
            action = Some(Action::Query {
                server,
                module: view.id(),
                request,
            });
        }
        ui.add_space(GAP);
    }
    action
}
