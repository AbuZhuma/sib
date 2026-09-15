use asiba_config::SummaryLayout;
use asiba_core::ServerState;
use egui::{Id, RichText, Ui};

use super::{connection, header, modules_table, select_tab};
use crate::components::{panel, panel_with_controls};
use crate::modules::{ModuleView, Tab, has_data};
use crate::pages::Action;
use crate::text;
use crate::theme::{GAP, Palette};

const EDIT_KEY: &str = "summary-layout-editing";

pub struct SummaryContext<'a> {
    pub server: &'a ServerState,
    pub views: &'a [Box<dyn ModuleView>],
    pub layout: &'a SummaryLayout,
}

pub fn show(ui: &mut Ui, ctx: &SummaryContext<'_>) -> Option<Action> {
    let server = ctx.server;
    let mut action = panel(ui, text::DETAIL_SECTION_CONNECTION, |ui| {
        connection::show(ui, server)
    });
    ui.add_space(GAP);
    let editing = edit_toggle(ui);
    let available: Vec<&dyn ModuleView> = ctx
        .views
        .iter()
        .map(Box::as_ref)
        .filter(|v| has_data(server, v.id()))
        .collect();
    let ids: Vec<&str> = available.iter().map(|v| v.id().0).collect();
    let mut layout = ctx.layout.clone();
    let mut changed = false;
    let visible: Vec<&dyn ModuleView> = layout
        .arrange(&ids)
        .into_iter()
        .filter(|id| !layout.is_hidden(id))
        .filter_map(|id| available.iter().copied().find(|v| v.id().0 == id))
        .collect();
    ui.columns(2, |columns| {
        for (index, view) in visible.iter().enumerate() {
            let column = &mut columns[index % 2];
            let id = view.id().0;
            if editing {
                panel_with_controls(
                    column,
                    view.title(),
                    |ui| changed |= layout_controls(ui, &mut layout, &ids, id),
                    |ui| view.summary(ui, server),
                );
            } else {
                panel_with_controls(
                    column,
                    view.title(),
                    |ui| open_button(ui, view.tab()),
                    |ui| view.summary(ui, server),
                );
            }
            column.add_space(GAP);
        }
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
    });
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
