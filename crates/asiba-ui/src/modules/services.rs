use asiba_core::{ModuleId, QueryRequest, ServerState};
use asiba_modules::services::{self, ServicesSnapshot, Unit, UnitOrigin};
use egui::{Id, RichText, Ui};

use super::{ModuleView, Tab, ViewAction, ViewShared, action_button};
use crate::components::{Sort, SortColumn, SortKey, Table, badge, sort_rows};
use crate::format;
use crate::text;
use crate::theme::{GAP, Palette};

const UNIT_SORTABLE: [SortColumn; 4] = [
    SortColumn::text(0),
    SortColumn::text(1),
    SortColumn::number(2),
    SortColumn::text(3),
];
const UNIT_DEFAULT_SORT: Sort = Sort::ascending(0);

#[derive(Debug, Clone, Copy, Default)]
struct Filters {
    only_problems: bool,
    only_custom: bool,
}

const SUMMARY_UNITS: usize = 8;

pub struct ServicesView;

impl ModuleView for ServicesView {
    fn id(&self) -> ModuleId {
        services::ID
    }

    fn title(&self) -> &'static str {
        text::MODULE_SERVICES
    }

    fn tab(&self) -> Tab {
        Tab::Services
    }

    fn summary(&self, ui: &mut Ui, server: &ServerState) {
        let p = Palette::current(ui.ctx());
        let Some(snapshot) = server.data::<ServicesSnapshot>(services::ID) else {
            return;
        };
        let failed: Vec<&Unit> = snapshot.failed().collect();
        ui.horizontal_wrapped(|ui| {
            ui.monospace(format!("{} {}", snapshot.active_count(), text::SVC_ACTIVE));
            let color = if failed.is_empty() {
                p.text
            } else {
                p.critical
            };
            ui.label(
                RichText::new(format!("{} {}", failed.len(), text::SVC_FAILED))
                    .monospace()
                    .color(color),
            );
        });
        for unit in failed.iter().take(5) {
            ui.label(RichText::new(format!("{} - {}", unit.name, unit.result)).color(p.critical));
        }
        let custom: Vec<&Unit> = snapshot
            .custom()
            .filter(|u| u.is_active())
            .take(SUMMARY_UNITS)
            .collect();
        if !custom.is_empty() {
            ui.add_space(GAP);
            let columns = [
                text::COL_MODULE,
                text::COL_STATUS,
                text::SVC_RESTARTS,
                text::COL_MESSAGE,
            ];
            Table::new("services-summary", &columns).show(ui, |ui| {
                for unit in custom {
                    ui.monospace(unit.short_name());
                    badge(ui, &unit.sub, p.ok);
                    ui.monospace(unit.restarts.to_string());
                    ui.label(RichText::new(&unit.description).color(p.text_secondary));
                    ui.end_row();
                }
            });
        }
    }

    fn page(&self, ui: &mut Ui, server: &ServerState, _shared: &ViewShared) -> Option<ViewAction> {
        let snapshot = server.data::<ServicesSnapshot>(services::ID)?;
        let id = Id::new(("services-filters", server.spec.id.as_str()));
        let mut filters: Filters = ui.ctx().data(|d| d.get_temp(id)).unwrap_or_default();
        ui.horizontal(|ui| {
            ui.checkbox(&mut filters.only_problems, text::SVC_ONLY_PROBLEMS);
            ui.checkbox(&mut filters.only_custom, text::SVC_ONLY_CUSTOM);
        });
        ui.ctx().data_mut(|d| d.insert_temp(id, filters));
        ui.add_space(GAP);
        let action = units_table(ui, snapshot, filters);
        if !snapshot.timers.is_empty() {
            ui.add_space(GAP);
            timers_table(ui, snapshot);
        }
        action
    }
}

fn units_table(ui: &mut Ui, snapshot: &ServicesSnapshot, filters: Filters) -> Option<ViewAction> {
    let p = Palette::current(ui.ctx());
    let mut action = None;
    let columns = [
        text::COL_MODULE,
        text::COL_STATUS,
        text::SVC_RESTARTS,
        "PID",
        text::COL_MESSAGE,
        "",
    ];
    let rows = snapshot.units.iter().filter(|u| {
        (!filters.only_problems || u.is_failed() || u.restarts > 0)
            && (!filters.only_custom || u.origin() == UnitOrigin::Custom)
    });
    let table = Table::new("services-units", &columns).sortable(&UNIT_SORTABLE, UNIT_DEFAULT_SORT);
    table.show_sorted(ui, |ui, sort| {
        let mut rows: Vec<&Unit> = rows.collect();
        sort_rows(&mut rows, sort, |unit, column| match column {
            0 => SortKey::text(unit.short_name()),
            1 => SortKey::text(&unit.sub),
            2 => SortKey::number(unit.restarts as f64),
            _ => SortKey::number(unit.main_pid as f64),
        });
        for unit in rows {
            ui.monospace(unit.short_name());
            let color = if unit.is_failed() {
                p.critical
            } else if unit.is_active() {
                p.ok
            } else {
                p.text_muted
            };
            badge(ui, &format!("{} / {}", unit.active, unit.sub), color);
            let restarts_color = if unit.restarts > 0 {
                p.warning
            } else {
                p.text_muted
            };
            ui.label(
                RichText::new(unit.restarts.to_string())
                    .monospace()
                    .color(restarts_color),
            );
            ui.monospace(if unit.main_pid > 0 {
                unit.main_pid.to_string()
            } else {
                "-".to_owned()
            });
            ui.label(&unit.description);
            ui.horizontal(|ui| {
                if ui.small_button(text::SVC_JOURNAL).clicked() {
                    action = Some(ViewAction::Query(QueryRequest::new(
                        services::QUERY_JOURNAL,
                        &unit.name,
                    )));
                }
                if let Some(next) = unit_buttons(ui, unit) {
                    action = Some(next);
                }
            });
            ui.end_row();
        }
    });
    action
}

fn timers_table(ui: &mut Ui, snapshot: &ServicesSnapshot) {
    let p = Palette::current(ui.ctx());
    ui.label(
        RichText::new(text::SVC_TIMERS.to_uppercase())
            .small()
            .color(p.text_secondary),
    );
    let columns = [text::COL_MODULE, text::SVC_NEXT, text::SVC_LAST, ""];
    Table::new("services-timers", &columns).show(ui, |ui| {
        for timer in &snapshot.timers {
            ui.monospace(&timer.name);
            ui.monospace(
                timer
                    .next
                    .map(format::date_time)
                    .unwrap_or_else(|| "-".to_owned()),
            );
            ui.monospace(
                timer
                    .last
                    .map(format::date_time)
                    .unwrap_or_else(|| "-".to_owned()),
            );
            ui.label(RichText::new(&timer.activates).color(p.text_muted));
            ui.end_row();
        }
    });
}

fn unit_buttons(ui: &mut Ui, unit: &Unit) -> Option<ViewAction> {
    if unit.is_active() {
        action_button(ui, text::ACT_RESTART, services::SPEC_RESTART, &unit.name)
            .or_else(|| action_button(ui, text::ACT_STOP, services::SPEC_STOP, &unit.name))
    } else {
        action_button(ui, text::ACT_START, services::SPEC_START, &unit.name)
    }
}
