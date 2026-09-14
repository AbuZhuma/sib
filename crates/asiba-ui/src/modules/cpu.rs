use asiba_core::{ModuleId, ServerState};
use asiba_modules::cpu::{self, CpuSnapshot};
use egui::{Grid, RichText, Ui, Vec2};

use super::{ModuleView, Tab};
use crate::components::{TimeSeriesPlot, Unit, meter, sparkline};
use crate::text;
use crate::theme::{GAP, Palette};

const SPARKLINE_SIZE: Vec2 = Vec2::new(160.0, 36.0);
const CORE_COLUMNS: usize = 4;

pub struct CpuView;

impl ModuleView for CpuView {
    fn id(&self) -> ModuleId {
        cpu::ID
    }

    fn title(&self) -> &'static str {
        text::MODULE_CPU
    }

    fn tab(&self) -> Tab {
        Tab::Resources
    }

    fn summary(&self, ui: &mut Ui, server: &ServerState) {
        let p = Palette::current(ui.ctx());
        let Some(snapshot) = server.data::<CpuSnapshot>(cpu::ID) else {
            return;
        };
        let busy = server.latest_value(cpu::KEY_TOTAL);
        ui.horizontal(|ui| {
            let value = busy
                .map(|v| format!("{v:.0}%"))
                .unwrap_or_else(|| "—".to_owned());
            ui.label(RichText::new(value).size(28.0).monospace());
            sparkline(
                ui,
                server.series.get(cpu::KEY_TOTAL),
                SPARKLINE_SIZE,
                p.chart[0],
                Some(100.0),
            );
        });
        ui.horizontal_wrapped(|ui| {
            ui.monospace(format!("{} {}", snapshot.core_count(), text::CPU_CORES));
            if let Some(freq) = snapshot.frequency_mhz {
                ui.monospace(format!("{freq:.0} MHz"));
            }
            if let Some(temp) = snapshot.temperature_c {
                ui.monospace(format!("{temp:.0}°C"));
            }
            if let Some(pressure) = snapshot.pressure {
                ui.monospace(format!("PSI {:.2}", pressure.some.avg10));
            }
        });
        if let Some(usage) = &snapshot.usage {
            ui.add_space(GAP);
            cores_grid(ui, usage.cores.iter().map(|c| c.busy));
        }
    }

    fn page(&self, ui: &mut Ui, server: &ServerState) {
        let p = Palette::current(ui.ctx());
        let Some(snapshot) = server.data::<CpuSnapshot>(cpu::ID) else {
            return;
        };
        let mut plot = TimeSeriesPlot::new("cpu-plot", Unit::Percent).height(180.0);
        let keys = [
            (cpu::KEY_TOTAL, text::CPU_TOTAL, p.chart[0]),
            (cpu::KEY_USER, text::CPU_USER, p.chart[1]),
            (cpu::KEY_SYSTEM, text::CPU_SYSTEM, p.chart[2]),
            (cpu::KEY_IOWAIT, text::CPU_IOWAIT, p.chart[3]),
            (cpu::KEY_STEAL, text::CPU_STEAL, p.chart[4]),
        ];
        for (key, label, color) in keys {
            if let Some(series) = server.series.get(key) {
                plot = plot.series(label, series, color);
            }
        }
        plot.show(ui);
        ui.add_space(GAP);
        if let Some(series) = server.series.get(cpu::KEY_TEMPERATURE) {
            TimeSeriesPlot::new("cpu-temperature", Unit::Celsius)
                .height(80.0)
                .series(text::CPU_TEMPERATURE, series, p.chart[3])
                .show(ui);
            ui.add_space(GAP);
        }
        if let Some(usage) = &snapshot.usage {
            core_meters(ui, usage.cores.iter().map(|c| c.busy));
        }
        ui.add_space(GAP);
        details(ui, snapshot);
    }
}

fn cores_grid(ui: &mut Ui, cores: impl Iterator<Item = f64>) {
    let p = Palette::current(ui.ctx());
    ui.horizontal_wrapped(|ui| {
        ui.spacing_mut().item_spacing = Vec2::new(2.0, 2.0);
        for busy in cores {
            let (rect, _) = ui.allocate_exact_size(Vec2::new(14.0, 14.0), egui::Sense::hover());
            let fill = p
                .accent
                .gamma_multiply((busy / 100.0).clamp(0.08, 1.0) as f32);
            ui.painter().rect_filled(rect, 0.0, p.bg_window);
            ui.painter().rect_filled(rect, 0.0, fill);
        }
    });
}

fn core_meters(ui: &mut Ui, cores: impl Iterator<Item = f64>) {
    let cores: Vec<f64> = cores.collect();
    Grid::new("cpu-cores")
        .num_columns(CORE_COLUMNS)
        .spacing([GAP * 2.0, 2.0])
        .show(ui, |ui| {
            for (index, busy) in cores.iter().enumerate() {
                ui.vertical(|ui| {
                    ui.set_width(150.0);
                    meter(ui, &format!("cpu{index}"), *busy, "");
                });
                if (index + 1) % CORE_COLUMNS == 0 {
                    ui.end_row();
                }
            }
        });
}

fn details(ui: &mut Ui, snapshot: &CpuSnapshot) {
    let p = Palette::current(ui.ctx());
    Grid::new("cpu-details")
        .num_columns(2)
        .spacing([16.0, 4.0])
        .show(ui, |ui| {
            let mut row = |label: &str, value: String| {
                ui.label(RichText::new(label).color(p.text_secondary));
                ui.monospace(value);
                ui.end_row();
            };
            row(text::CPU_CORES, snapshot.core_count().to_string());
            if let Some(freq) = snapshot.frequency_mhz {
                row(text::CPU_FREQUENCY, format!("{freq:.0} MHz"));
            }
            if let Some(temp) = snapshot.temperature_c {
                row(text::CPU_TEMPERATURE, format!("{temp:.1}°C"));
            }
            if let Some(pressure) = snapshot.pressure {
                let line = pressure.some;
                row(
                    text::CPU_PRESSURE,
                    format!("{:.2} / {:.2} / {:.2}", line.avg10, line.avg60, line.avg300),
                );
            }
        });
}
