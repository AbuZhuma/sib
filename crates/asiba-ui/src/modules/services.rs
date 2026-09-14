use asiba_core::{ModuleId, QueryRequest, ServerState};
use asiba_modules::services::{self, ServicesSnapshot, Unit, UnitOrigin};
use egui::{Id, RichText, Ui};

use super::{ModuleView, Tab, ViewAction};
use crate::components::{Table, badge};
use crate::format;
use crate::text;
use crate::theme::{GAP, Palette};

#[derive(Debug, Clone, Copy, Default)]
struct Filters {
    only_problems: bool,
    only_custom: bool,
}

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
            ui.label(RichText::new(format!("{} — {}", unit.name, unit.result)).color(p.critical));
        }
    }

    fn page(&self, ui: &mut Ui, server: &ServerState) -> Option<ViewAction> {
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
    Table::new("services-units", &columns).show(ui, |ui| {
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
                "—".to_owned()
            });
            ui.label(&unit.description);
            if ui.small_button(text::SVC_JOURNAL).clicked() {
                action = Some(ViewAction::Query(QueryRequest::new(
                    services::QUERY_JOURNAL,
                    &unit.name,
                )));
            }
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
                    .unwrap_or_else(|| "—".to_owned()),
            );
            ui.monospace(
                timer
                    .last
                    .map(format::date_time)
                    .unwrap_or_else(|| "—".to_owned()),
            );
            ui.label(RichText::new(&timer.activates).color(p.text_muted));
            ui.end_row();
        }
    });
}
