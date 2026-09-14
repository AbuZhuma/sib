mod connection;
mod header;
mod modules_table;

use asiba_core::ServerState;
use egui::{Id, RichText, ScrollArea, Ui};

use super::Action;
use crate::components::panel;
use crate::modules::{ModuleView, Tab, has_data};
use crate::text;
use crate::theme::{GAP, Palette};

const TAB_KEY: &str = "server-detail-tab";

pub fn show(ui: &mut Ui, server: &ServerState, views: &[Box<dyn ModuleView>]) -> Option<Action> {
    let mut action = header::show(ui, server);
    let tabs = visible_tabs(server, views);
    let mut tab = current_tab(ui, &tabs);
    tab_bar(ui, &tabs, &mut tab);
    ui.ctx().data_mut(|d| d.insert_temp(Id::new(TAB_KEY), tab));
    ScrollArea::vertical().show(ui, |ui| match tab {
        Tab::Summary => {
            if let Some(next) = summary(ui, server, views) {
                action = Some(next);
            }
        }
        other => pages_for(ui, server, views, other),
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

fn tab_bar(ui: &mut Ui, tabs: &[Tab], current: &mut Tab) {
    let p = Palette::current(ui.ctx());
    ui.horizontal(|ui| {
        for tab in tabs {
            let selected = tab == current;
            let color = if selected { p.text } else { p.text_secondary };
            if ui
                .selectable_label(selected, RichText::new(tab.label()).color(color))
                .clicked()
            {
                *current = *tab;
            }
        }
    });
    ui.add_space(GAP);
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

fn pages_for(ui: &mut Ui, server: &ServerState, views: &[Box<dyn ModuleView>], tab: Tab) {
    for view in views
        .iter()
        .filter(|v| v.tab() == tab && has_data(server, v.id()))
    {
        panel(ui, view.title(), |ui| view.page(ui, server));
        ui.add_space(GAP);
    }
}
