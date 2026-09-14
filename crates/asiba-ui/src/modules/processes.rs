use asiba_core::{ModuleId, ServerState};
use asiba_modules::processes::{self, Process, ProcessSnapshot};
use egui::{Id, RichText, TextEdit, Ui};
use egui_extras::{Column, TableBuilder};

use super::{ModuleView, Tab, ViewAction};
use crate::format;
use crate::text;
use crate::theme::{GAP, Palette, ROW_HEIGHT};

const SUMMARY_TOP: usize = 5;
const CMD_MAX_CHARS: usize = 120;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
enum SortBy {
    #[default]
    Cpu,
    Memory,
    Read,
    Write,
    Pid,
    Name,
}

#[derive(Debug, Clone, Default)]
struct TableState {
    sort: SortBy,
    filter: String,
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

    fn summary(&self, ui: &mut Ui, server: &ServerState) {
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
        egui::Grid::new("processes-top")
            .num_columns(3)
            .spacing([12.0, 2.0])
            .show(ui, |ui| {
                for process in top.iter().take(SUMMARY_TOP) {
                    ui.monospace(format!("{:>5.1}%", process.cpu_pct.unwrap_or(0.0)));
                    ui.monospace(format::bytes(process.rss_bytes));
                    ui.label(truncate(process.display_name(), 48));
                    ui.end_row();
                }
            });
    }

    fn page(&self, ui: &mut Ui, server: &ServerState) -> Option<ViewAction> {
        let snapshot = server.data::<ProcessSnapshot>(processes::ID)?;
        let id = Id::new(("processes-table", server.spec.id.as_str()));
        let mut state: TableState = ui.ctx().data(|d| d.get_temp(id)).unwrap_or_default();
        toolbar(ui, &mut state);
        ui.add_space(GAP);
        let rows = filtered_sorted(snapshot, &state);
        table(ui, snapshot, &rows, &mut state);
        ui.ctx().data_mut(|d| d.insert_temp(id, state));
        None
    }
}

fn toolbar(ui: &mut Ui, state: &mut TableState) {
    ui.horizontal(|ui| {
        ui.add(
            TextEdit::singleline(&mut state.filter)
                .hint_text(text::PROC_FILTER)
                .desired_width(240.0),
        );
        ui.label(text::PROC_SORT);
        for (sort, label) in sort_options() {
            ui.selectable_value(&mut state.sort, sort, label);
        }
    });
}

fn sort_options() -> [(SortBy, &'static str); 6] {
    [
        (SortBy::Cpu, "CPU"),
        (SortBy::Memory, "RAM"),
        (SortBy::Read, "R"),
        (SortBy::Write, "W"),
        (SortBy::Pid, "PID"),
        (SortBy::Name, text::COL_NAME),
    ]
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
    let by_f64 =
        |a: Option<f64>, b: Option<f64>| b.partial_cmp(&a).unwrap_or(std::cmp::Ordering::Equal);
    rows.sort_by(|a, b| match state.sort {
        SortBy::Cpu => by_f64(a.cpu_pct, b.cpu_pct),
        SortBy::Memory => b.rss_bytes.cmp(&a.rss_bytes),
        SortBy::Read => by_f64(a.read_bps, b.read_bps),
        SortBy::Write => by_f64(a.write_bps, b.write_bps),
        SortBy::Pid => a.pid.cmp(&b.pid),
        SortBy::Name => a.display_name().cmp(b.display_name()),
    });
    rows
}

fn table(ui: &mut Ui, snapshot: &ProcessSnapshot, rows: &[&Process], state: &mut TableState) {
    let p = Palette::current(ui.ctx());
    let headers = [
        "PID",
        text::PROC_USER,
        "CPU",
        "RAM",
        "R/s",
        "W/s",
        text::PROC_STATE,
        text::PROC_UPTIME,
        text::PROC_COMMAND,
    ];
    let body_height = ui.available_height();
    TableBuilder::new(ui)
        .striped(true)
        .column(Column::exact(64.0))
        .column(Column::exact(90.0))
        .column(Column::exact(64.0))
        .column(Column::exact(90.0))
        .column(Column::exact(90.0))
        .column(Column::exact(90.0))
        .column(Column::exact(40.0))
        .column(Column::exact(80.0))
        .column(Column::remainder().clip(true))
        .min_scrolled_height(body_height)
        .header(ROW_HEIGHT, |mut header| {
            for label in headers {
                header.col(|ui| {
                    ui.label(
                        RichText::new(label.to_uppercase())
                            .small()
                            .color(p.text_secondary),
                    );
                });
            }
        })
        .body(|body| {
            body.rows(ROW_HEIGHT, rows.len(), |mut row| {
                let process = rows[row.index()];
                row_cells(&mut row, snapshot, process, &p);
            });
        });
    let _ = state;
}

fn row_cells(
    row: &mut egui_extras::TableRow<'_, '_>,
    snapshot: &ProcessSnapshot,
    process: &Process,
    p: &Palette,
) {
    let mono = |row: &mut egui_extras::TableRow<'_, '_>, value: String| {
        row.col(|ui| {
            ui.monospace(value);
        });
    };
    mono(row, process.pid.to_string());
    mono(row, process.user.clone());
    mono(
        row,
        process
            .cpu_pct
            .map(|v| format!("{v:.1}%"))
            .unwrap_or_else(|| "—".to_owned()),
    );
    mono(
        row,
        format!(
            "{} ({:.1}%)",
            format::bytes(process.rss_bytes),
            snapshot.memory_pct(process)
        ),
    );
    mono(
        row,
        process
            .read_bps
            .map(format::bytes_per_second)
            .unwrap_or_else(|| "—".to_owned()),
    );
    mono(
        row,
        process
            .write_bps
            .map(format::bytes_per_second)
            .unwrap_or_else(|| "—".to_owned()),
    );
    row.col(|ui| {
        let color = if process.is_zombie() {
            p.warning
        } else {
            p.text
        };
        ui.label(
            RichText::new(process.state.to_string())
                .monospace()
                .color(color),
        );
    });
    mono(
        row,
        format::duration_short(snapshot.started_secs_ago(process)),
    );
    row.col(|ui| {
        ui.label(truncate(process.display_name(), CMD_MAX_CHARS));
    });
}

fn truncate(value: &str, max_chars: usize) -> String {
    if value.chars().count() <= max_chars {
        return value.to_owned();
    }
    let cut: String = value.chars().take(max_chars).collect();
    format!("{cut}…")
}
