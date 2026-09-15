mod entries;

use asiba_core::{ModuleId, ServerState};
use asiba_modules::logs::{self, LogsSnapshot};
use chrono::Duration;
use egui::{Id, RichText, TextEdit, Ui};

use super::{ModuleView, Tab, ViewAction, ViewShared};
use crate::components::Table;
use crate::format;
use crate::text;
use crate::theme::{GAP, Palette};

const MAX_GROUP_ROWS: usize = 300;
const FILTER_WIDTH: f32 = 280.0;

#[derive(Debug, Clone, Default)]
struct LogFilter {
    query: String,
    grouped: bool,
}

pub struct LogsView;

impl ModuleView for LogsView {
    fn id(&self) -> ModuleId {
        logs::ID
    }

    fn title(&self) -> &'static str {
        text::MODULE_LOGS
    }

    fn tab(&self) -> Tab {
        Tab::Logs
    }

    fn summary(&self, ui: &mut Ui, server: &ServerState) {
        let p = Palette::current(ui.ctx());
        let Some(snapshot) = server.data::<LogsSnapshot>(logs::ID) else {
            return;
        };
        let hour_errors = snapshot.count_since(Duration::hours(1), 3);
        let hour_warnings = snapshot.count_since(Duration::hours(1), 4) - hour_errors;
        let day_errors = snapshot.count_since(Duration::hours(24), 3);
        ui.horizontal_wrapped(|ui| {
            let color = if hour_errors > 0 { p.critical } else { p.text };
            ui.label(
                RichText::new(format!(
                    "{hour_errors} {} {}",
                    text::LOG_ERRORS,
                    text::LOG_LAST_HOUR
                ))
                .monospace()
                .color(color),
            );
            ui.monospace(format!(
                "{hour_warnings} {} {}",
                text::LOG_WARNINGS,
                text::LOG_LAST_HOUR
            ));
            ui.monospace(format!(
                "{day_errors} {} {}",
                text::LOG_ERRORS,
                text::LOG_LAST_DAY
            ));
        });
        for group in snapshot.groups().iter().filter(|g| g.priority <= 3).take(5) {
            ui.label(
                RichText::new(format!(
                    "×{} {}: {}",
                    group.count, group.source, group.message
                ))
                .small()
                .color(p.text_secondary),
            );
        }
    }

    fn page(&self, ui: &mut Ui, server: &ServerState, _shared: &ViewShared) -> Option<ViewAction> {
        let snapshot = server.data::<LogsSnapshot>(logs::ID)?;
        let id = Id::new(("logs-filter", server.spec.id.as_str()));
        let mut filter: LogFilter = ui.ctx().data(|d| d.get_temp(id)).unwrap_or_default();
        toolbar(ui, &mut filter, snapshot);
        ui.ctx().data_mut(|d| d.insert_temp(id, filter.clone()));
        ui.add_space(GAP);
        if filter.grouped {
            grouped_table(ui, snapshot, &filter.query);
            return None;
        }
        entries::table(ui, server.spec.id.as_str(), snapshot, &filter.query)
    }
}

fn toolbar(ui: &mut Ui, filter: &mut LogFilter, snapshot: &LogsSnapshot) {
    let p = Palette::current(ui.ctx());
    ui.horizontal(|ui| {
        ui.add(
            TextEdit::singleline(&mut filter.query)
                .hint_text(text::LOG_FILTER)
                .desired_width(FILTER_WIDTH),
        );
        ui.checkbox(&mut filter.grouped, text::LOG_GROUPED);
        ui.label(
            RichText::new(format!("{} {}", snapshot.entries.len(), text::LOG_ENTRIES))
                .color(p.text_muted),
        );
        if !snapshot.has_older {
            ui.label(RichText::new(text::LOG_ALL_LOADED).color(p.text_muted));
        }
    });
}

pub(super) fn matches(query: &str, source: &str, message: &str) -> bool {
    let needle = query.trim().to_lowercase();
    needle.is_empty()
        || source.to_lowercase().contains(&needle)
        || message.to_lowercase().contains(&needle)
}

fn grouped_table(ui: &mut Ui, snapshot: &LogsSnapshot, query: &str) {
    let p = Palette::current(ui.ctx());
    let columns = [
        text::LOG_COUNT,
        text::COL_TIME,
        text::LOG_LEVEL,
        text::LOG_SOURCE,
        text::COL_MESSAGE,
    ];
    Table::new("logs-groups", &columns).show(ui, |ui| {
        let rows = snapshot
            .groups()
            .into_iter()
            .filter(|g| matches(query, &g.source, &g.message))
            .take(MAX_GROUP_ROWS);
        for group in rows {
            ui.monospace(format!("×{}", group.count));
            ui.monospace(format::clock(group.last_at));
            let label = match group.priority {
                0..=2 => "crit",
                3 => "err",
                4 => "warning",
                _ => "info",
            };
            ui.label(
                RichText::new(label)
                    .monospace()
                    .color(entries::priority_color(&p, group.priority)),
            );
            ui.monospace(&group.source);
            ui.label(&group.message);
            ui.end_row();
        }
    });
}
