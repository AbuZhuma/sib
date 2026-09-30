use egui::scroll_area::ScrollBarVisibility;
use egui::{Color32, Id, RichText, Ui};
use egui_extras::{Column, TableBuilder, TableRow};
use sib_modules::logs::{LogEntry, LogsSnapshot};

use super::matches;
use crate::format;
use crate::modules::ViewAction;
use crate::text;
use crate::theme::{LONG_TABLE_FRACTION, Palette, ROW_HEIGHT};

const TIME_WIDTH: f32 = 72.0;
const LEVEL_WIDTH: f32 = 64.0;
const SOURCE_WIDTH: f32 = 180.0;
const BACKFILL_KEY: &str = "logs-backfill";
const PENDING_TIMEOUT_SECS: f64 = 30.0;

pub fn priority_color(p: &Palette, priority: u8) -> Color32 {
    match priority {
        0..=3 => p.critical,
        4 => p.warning,
        _ => p.text_secondary,
    }
}

const HEADERS: [&str; 4] = [
    text::COL_TIME,
    text::LOG_LEVEL,
    text::LOG_SOURCE,
    text::COL_MESSAGE,
];

pub fn table(
    ui: &mut Ui,
    server: &str,
    snapshot: &LogsSnapshot,
    query: &str,
) -> Option<ViewAction> {
    let p = Palette::current(ui.ctx());
    let rows: Vec<&LogEntry> = snapshot
        .entries
        .iter()
        .rev()
        .filter(|e| matches(query, e.source(), &e.message))
        .collect();
    let body_height = ui.available_height() * LONG_TABLE_FRACTION;
    let mut is_end_visible = false;
    TableBuilder::new(ui)
        .scroll_bar_visibility(ScrollBarVisibility::AlwaysHidden)
        .striped(true)
        .column(Column::exact(TIME_WIDTH))
        .column(Column::exact(LEVEL_WIDTH))
        .column(Column::exact(SOURCE_WIDTH))
        .column(Column::remainder().clip(true))
        .min_scrolled_height(body_height)
        .header(ROW_HEIGHT, |mut header| {
            header_cells(&mut header, &HEADERS, &p)
        })
        .body(|body| {
            body.rows(ROW_HEIGHT, rows.len(), |mut row| {
                let is_last = row.index() + 1 == rows.len();
                let entry = rows[row.index()];
                if row_cells(&mut row, entry, &p) && is_last {
                    is_end_visible = true;
                }
            });
        });
    request_older(ui, server, snapshot, is_end_visible)
}

fn header_cells(header: &mut TableRow<'_, '_>, labels: &[&str], p: &Palette) {
    for label in labels {
        header.col(|ui| {
            ui.label(
                RichText::new(label.to_uppercase())
                    .small()
                    .color(p.text_secondary),
            );
        });
    }
}

fn row_cells(row: &mut TableRow<'_, '_>, entry: &LogEntry, p: &Palette) -> bool {
    let mut is_visible = false;
    row.col(|ui| {
        is_visible = ui.is_rect_visible(ui.max_rect());
        ui.monospace(format::clock(entry.at));
    });
    row.col(|ui| {
        ui.label(
            RichText::new(entry.priority_label())
                .monospace()
                .color(priority_color(p, entry.priority)),
        );
    });
    row.col(|ui| {
        ui.monospace(entry.source());
    });
    row.col(|ui| {
        ui.label(entry.message_short());
    });
    is_visible
}

fn request_older(
    ui: &mut Ui,
    server: &str,
    snapshot: &LogsSnapshot,
    is_end_visible: bool,
) -> Option<ViewAction> {
    let p = Palette::current(ui.ctx());
    let id = Id::new((BACKFILL_KEY, server));
    let now = ui.input(|i| i.time);
    let requested: Option<(usize, f64)> = ui.ctx().data(|d| d.get_temp(id));
    let is_pending = requested
        .is_some_and(|(len, at)| len == snapshot.entries.len() && now - at < PENDING_TIMEOUT_SECS);
    if is_pending && snapshot.has_older {
        ui.label(RichText::new(text::LOG_LOADING_OLDER).color(p.text_muted));
    }
    if !is_end_visible || !snapshot.has_older || is_pending {
        return None;
    }
    ui.ctx()
        .data_mut(|d| d.insert_temp(id, (snapshot.entries.len(), now)));
    Some(ViewAction::Backfill)
}
