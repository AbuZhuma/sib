mod row;

use egui::scroll_area::ScrollBarVisibility;
use egui::{Id, RichText, TextEdit, Ui};
use egui_extras::{Column, TableBuilder};
use sib_core::{ModuleId, ServerState};
use sib_modules::processes::{self, Process, ProcessSnapshot};

use super::{ModuleView, Tab, ViewAction, ViewShared};
use crate::components::{Sort, SortColumn, SortKey, sort_header, sort_rows};
use crate::format;
use crate::text;
use crate::theme::{GAP, LONG_TABLE_FRACTION, Palette, ROW_HEIGHT};

const SUMMARY_TOP: usize = 8;
const ACTIONS_WIDTH: f32 = 104.0;
const USER_WIDTH: f32 = 130.0;
const MEMORY_WIDTH: f32 = 130.0;
const PID_WIDTH: f32 = 64.0;
const CPU_WIDTH: f32 = 64.0;
const IO_WIDTH: f32 = 90.0;
const STATE_WIDTH: f32 = 40.0;
const UPTIME_WIDTH: f32 = 80.0;
const HEADERS: [&str; 10] = [
    "PID",
    text::PROC_USER,
    "CPU",
    "RAM",
    "R/s",
    "W/s",
    text::PROC_STATE,
    text::PROC_UPTIME,
    "",
    text::PROC_COMMAND,
];
const SUMMARY_COMMAND_CHARS: usize = 48;
const FIXED_COLUMNS: [f32; 9] = [
    PID_WIDTH,
    USER_WIDTH,
    CPU_WIDTH,
    MEMORY_WIDTH,
    IO_WIDTH,
    IO_WIDTH,
    STATE_WIDTH,
    UPTIME_WIDTH,
    ACTIONS_WIDTH,
];

const COLUMN_PID: usize = 0;
const COLUMN_USER: usize = 1;
const COLUMN_CPU: usize = 2;
const COLUMN_MEMORY: usize = 3;
const COLUMN_READ: usize = 4;
const COLUMN_WRITE: usize = 5;
const COLUMN_STATE: usize = 6;
const COLUMN_UPTIME: usize = 7;
const COLUMN_COMMAND: usize = 9;
const SORTABLE: [SortColumn; 9] = [
    SortColumn {
        index: COLUMN_PID,
        descending_first: false,
    },
    SortColumn::text(COLUMN_USER),
    SortColumn::number(COLUMN_CPU),
    SortColumn::number(COLUMN_MEMORY),
    SortColumn::number(COLUMN_READ),
    SortColumn::number(COLUMN_WRITE),
    SortColumn::text(COLUMN_STATE),
    SortColumn::number(COLUMN_UPTIME),
    SortColumn::text(COLUMN_COMMAND),
];
const DEFAULT_SORT: Sort = Sort::descending(COLUMN_CPU);

#[derive(Debug, Clone)]
struct TableState {
    sort: Sort,
    filter: String,
}

impl Default for TableState {
    fn default() -> Self {
        Self {
            sort: DEFAULT_SORT,
            filter: String::new(),
        }
    }
}

pub struct ProcessesView;

impl ModuleView for ProcessesView {
    fn id(&self) -> ModuleId {
        processes::ID
    }

    fn title(&self) -> &'static str {
        text::MODULE_PROCESSES
    }

    fn tab(&self) -> Tab {
        Tab::Processes
    }

    fn summary(&self, ui: &mut Ui, server: &ServerState, _shared: &ViewShared) {
        let p = Palette::current(ui.ctx());
        let Some(snapshot) = server.data::<ProcessSnapshot>(processes::ID) else {
            return;
        };
        ui.horizontal_wrapped(|ui| {
            ui.monospace(format!("{} {}", snapshot.processes.len(), text::PROC_TOTAL));
            ui.monospace(format!(
                "{} {}",
                snapshot.running_count(),
                text::PROC_RUNNING
            ));
            let zombies = snapshot.zombie_count();
            let color = if zombies > 0 { p.warning } else { p.text };
            ui.label(
                RichText::new(format!("{zombies} {}", text::PROC_ZOMBIES))
                    .monospace()
                    .color(color),
            );
        });
        top_grid(ui, snapshot);
    }

    fn page(&self, ui: &mut Ui, server: &ServerState, _shared: &ViewShared) -> Option<ViewAction> {
        let snapshot = server.data::<ProcessSnapshot>(processes::ID)?;
        let id = Id::new(("processes-table", server.spec.id.as_str()));
        let mut state: TableState = ui.ctx().data(|d| d.get_temp(id)).unwrap_or_default();
        toolbar(ui, &mut state);
        ui.add_space(GAP);
        let rows = filtered_sorted(snapshot, &state);
        let action = table(ui, snapshot, &rows, &mut state);
        ui.ctx().data_mut(|d| d.insert_temp(id, state));
        action
    }
}

fn toolbar(ui: &mut Ui, state: &mut TableState) {
    ui.add(
        TextEdit::singleline(&mut state.filter)
            .hint_text(text::PROC_FILTER)
            .desired_width(240.0),
    );
}

fn sort_key(process: &Process, snapshot: &ProcessSnapshot, column: usize) -> SortKey {
    match column {
        COLUMN_PID => SortKey::number(process.pid),
        COLUMN_USER => SortKey::text(&process.user),
        COLUMN_CPU => SortKey::optional(process.cpu_pct),
        COLUMN_MEMORY => SortKey::number(process.rss_bytes as f64),
        COLUMN_READ => SortKey::optional(process.read_bps),
        COLUMN_WRITE => SortKey::optional(process.write_bps),
        COLUMN_STATE => SortKey::text(&process.state.to_string()),
        COLUMN_UPTIME => SortKey::number(snapshot.started_secs_ago(process)),
        _ => SortKey::text(process.display_name()),
    }
}

fn filtered_sorted<'a>(snapshot: &'a ProcessSnapshot, state: &TableState) -> Vec<&'a Process> {
    let needle = state.filter.trim().to_lowercase();
    let mut rows: Vec<&Process> = snapshot
        .processes
        .iter()
        .filter(|p| {
            needle.is_empty()
                || p.display_name().to_lowercase().contains(&needle)
                || p.user.contains(&needle)
        })
        .collect();
    sort_rows(&mut rows, state.sort, |process, column| {
        sort_key(process, snapshot, column)
    });
    rows
}

fn top_by_cpu(snapshot: &ProcessSnapshot) -> Vec<&Process> {
    let mut top: Vec<&Process> = snapshot
        .processes
        .iter()
        .filter(|p| p.cpu_pct.is_some())
        .collect();
    top.sort_by(|a, b| {
        b.cpu_pct
            .partial_cmp(&a.cpu_pct)
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    top
}

fn top_grid(ui: &mut Ui, snapshot: &ProcessSnapshot) {
    egui::Grid::new("processes-top")
        .num_columns(3)
        .spacing([12.0, 2.0])
        .show(ui, |ui| {
            for process in top_by_cpu(snapshot).iter().take(SUMMARY_TOP) {
                ui.monospace(format!("{:>5.1}%", process.cpu_pct.unwrap_or(0.0)));
                ui.monospace(format::bytes(process.rss_bytes));
                ui.label(row::truncate(process.display_name(), SUMMARY_COMMAND_CHARS));
                ui.end_row();
            }
        });
}

fn table(
    ui: &mut Ui,
    snapshot: &ProcessSnapshot,
    rows: &[&Process],
    state: &mut TableState,
) -> Option<ViewAction> {
    let p = Palette::current(ui.ctx());
    let body_height = ui.available_height() * LONG_TABLE_FRACTION;
    let mut action = None;
    let mut builder = TableBuilder::new(ui)
        .scroll_bar_visibility(ScrollBarVisibility::AlwaysHidden)
        .striped(true);
    for width in FIXED_COLUMNS {
        builder = builder.column(Column::exact(width));
    }
    builder
        .column(Column::remainder().clip(true))
        .min_scrolled_height(body_height)
        .header(ROW_HEIGHT, |mut header| {
            for (index, label) in HEADERS.into_iter().enumerate() {
                header.col(|ui| header_cell(ui, index, label, &mut state.sort));
            }
        })
        .body(|body| {
            body.rows(ROW_HEIGHT, rows.len(), |mut row| {
                let process = rows[row.index()];
                if let Some(next) = row::cells(&mut row, snapshot, process, &p) {
                    action = Some(next);
                }
            });
        });
    action
}

fn header_cell(ui: &mut Ui, index: usize, label: &str, sort: &mut Sort) {
    let p = Palette::current(ui.ctx());
    let Some(column) = SORTABLE.iter().find(|c| c.index == index) else {
        ui.label(
            RichText::new(label.to_uppercase())
                .small()
                .color(p.text_secondary),
        );
        return;
    };
    let mut current = Some(*sort);
    sort_header(ui, label, *column, &mut current);
    if let Some(next) = current {
        *sort = next;
    }
}
