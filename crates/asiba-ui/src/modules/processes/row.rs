use asiba_modules::processes::{self, Process, ProcessSnapshot};
use egui::{RichText, Ui};
use egui_extras::TableRow;

use crate::format;
use crate::modules::{ViewAction, action_button};
use crate::text;
use crate::theme::Palette;

const CMD_MAX_CHARS: usize = 120;

pub fn cells(
    row: &mut TableRow<'_, '_>,
    snapshot: &ProcessSnapshot,
    process: &Process,
    p: &Palette,
) -> Option<ViewAction> {
    for value in metric_cells(snapshot, process) {
        row.col(|ui| {
            ui.monospace(value);
        });
    }
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
    row.col(|ui| {
        ui.monospace(format::duration_short(snapshot.started_secs_ago(process)));
    });
    let mut action = None;
    row.col(|ui| {
        action = process_buttons(ui, process);
    });
    row.col(|ui| {
        ui.label(truncate(process.display_name(), CMD_MAX_CHARS));
    });
    action
}

fn metric_cells(snapshot: &ProcessSnapshot, process: &Process) -> [String; 6] {
    let or_dash = |value: Option<String>| value.unwrap_or_else(|| "-".to_owned());
    [
        process.pid.to_string(),
        process.user.clone(),
        or_dash(process.cpu_pct.map(|v| format!("{v:.1}%"))),
        format!(
            "{} ({:.1}%)",
            format::bytes(process.rss_bytes),
            snapshot.memory_pct(process)
        ),
        or_dash(process.read_bps.map(format::bytes_per_second)),
        or_dash(process.write_bps.map(format::bytes_per_second)),
    ]
}

fn process_buttons(ui: &mut Ui, process: &Process) -> Option<ViewAction> {
    let pid = process.pid.to_string();
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = 2.0;
        action_button(ui, text::ACT_TERM, processes::SPEC_TERMINATE, &pid)
            .or_else(|| action_button(ui, text::ACT_KILL, processes::SPEC_KILL, &pid))
    })
    .inner
}

pub fn truncate(value: &str, max_chars: usize) -> String {
    if value.chars().count() <= max_chars {
        return value.to_owned();
    }
    let cut: String = value.chars().take(max_chars).collect();
    format!("{cut}…")
}
