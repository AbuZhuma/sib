use asiba_core::{ModuleId, ServerState};
use asiba_modules::memory::{self, MemorySnapshot};
use egui::{Grid, RichText, Ui};

use super::{ModuleView, Tab, ViewAction, ViewShared};
use crate::components::{TimeSeriesPlot, Unit, meter, sparkline_fill};
use crate::format;
use crate::text;
use crate::theme::{GAP, Palette, SUMMARY_RATE_HEIGHT};

pub struct MemoryView;

impl ModuleView for MemoryView {
    fn id(&self) -> ModuleId {
        memory::ID
    }

    fn title(&self) -> &'static str {
        text::MODULE_MEMORY
    }

    fn tab(&self) -> Tab {
        Tab::Resources
    }

    fn summary(&self, ui: &mut Ui, server: &ServerState) {
        let Some(snapshot) = server.data::<MemorySnapshot>(memory::ID) else {
            return;
        };
        let detail = format!(
            "{} / {}",
            format::bytes(snapshot.used_bytes()),
            format::bytes(snapshot.total_bytes)
        );
        meter(ui, text::MEM_USED, snapshot.used_pct(), &detail);
        if snapshot.swap_total_bytes > 0 {
            let swap = format!(
                "{} / {}",
                format::bytes(snapshot.swap_used_bytes()),
                format::bytes(snapshot.swap_total_bytes)
            );
            meter(ui, text::MEM_SWAP, snapshot.swap_used_pct(), &swap);
        }
        let p = Palette::current(ui.ctx());
        ui.add_space(GAP);
        let series = server.series.get(memory::KEY_USED_PCT);
        sparkline_fill(ui, series, SUMMARY_RATE_HEIGHT, p.chart[0], Some(100.0));
    }

    fn page(&self, ui: &mut Ui, server: &ServerState, _shared: &ViewShared) -> Option<ViewAction> {
        let p = Palette::current(ui.ctx());
        let snapshot = server.data::<MemorySnapshot>(memory::ID)?;
        self.summary(ui, server);
        ui.add_space(GAP);
        let mut plot = TimeSeriesPlot::new("memory-plot", Unit::Bytes);
        let keys = [
            (memory::KEY_USED_BYTES, text::MEM_USED, p.chart[0]),
            (memory::KEY_CACHED_BYTES, text::MEM_CACHED, p.chart[1]),
            (memory::KEY_AVAILABLE_BYTES, text::MEM_AVAILABLE, p.chart[2]),
        ];
        for (key, label, color) in keys {
            if let Some(series) = server.series.get(key) {
                plot = plot.series(label, series, color);
            }
        }
        plot.show(ui);
        ui.add_space(GAP);
        details(ui, snapshot);
        None
    }
}

fn details(ui: &mut Ui, snapshot: &MemorySnapshot) {
    let p = Palette::current(ui.ctx());
    Grid::new("memory-details")
        .num_columns(2)
        .spacing([16.0, 4.0])
        .show(ui, |ui| {
            let mut row = |label: &str, value: String| {
                ui.label(RichText::new(label).color(p.text_secondary));
                ui.monospace(value);
                ui.end_row();
            };
            row(text::MEM_TOTAL, format::bytes(snapshot.total_bytes));
            row(text::MEM_AVAILABLE, format::bytes(snapshot.available_bytes));
            row(text::MEM_FREE, format::bytes(snapshot.free_bytes));
            row(text::MEM_CACHED, format::bytes(snapshot.cached_bytes));
            row(text::MEM_BUFFERS, format::bytes(snapshot.buffers_bytes));
            row(text::MEM_SHMEM, format::bytes(snapshot.shmem_bytes));
            row(text::MEM_DIRTY, format::bytes(snapshot.dirty_bytes));
            if let (Some(swap_in), Some(swap_out)) = (snapshot.swap_in_ps, snapshot.swap_out_ps) {
                row(
                    text::MEM_SWAP_IO,
                    format!("{swap_in:.0} / {swap_out:.0} стр/с"),
                );
            }
            if let Some(pressure) = snapshot.pressure {
                let line = pressure.some;
                row(
                    text::MEM_PRESSURE,
                    format!("{:.2} / {:.2} / {:.2}", line.avg10, line.avg60, line.avg300),
                );
            }
            row(text::MEM_OOM, snapshot.oom_kills.to_string());
        });
}
