use asiba_core::{ModuleId, ServerState};
use asiba_modules::gpu::{self, Gpu, GpuSnapshot};
use egui::{RichText, Ui};

use super::{ModuleView, Tab, ViewAction};
use crate::components::{Table, TimeSeriesPlot, Unit, meter};
use crate::format;
use crate::text;
use crate::theme::{GAP, Palette, SECONDARY_PLOT_HEIGHT};

pub struct GpuView;

impl ModuleView for GpuView {
    fn id(&self) -> ModuleId {
        gpu::ID
    }

    fn title(&self) -> &'static str {
        text::MODULE_GPU
    }

    fn tab(&self) -> Tab {
        Tab::Gpu
    }

    fn summary(&self, ui: &mut Ui, server: &ServerState) {
        let Some(snapshot) = server.data::<GpuSnapshot>(gpu::ID) else {
            return;
        };
        for gpu in &snapshot.gpus {
            gpu_line(ui, gpu);
        }
    }

    fn page(&self, ui: &mut Ui, server: &ServerState) -> Option<ViewAction> {
        let p = Palette::current(ui.ctx());
        let snapshot = server.data::<GpuSnapshot>(gpu::ID)?;
        for gpu in &snapshot.gpus {
            gpu_line(ui, gpu);
        }
        ui.add_space(GAP);
        plot(ui, server, snapshot, "util_pct", Unit::Percent, &p, None);
        plot(
            ui,
            server,
            snapshot,
            "mem_pct",
            Unit::Percent,
            &p,
            Some(SECONDARY_PLOT_HEIGHT),
        );
        plot(
            ui,
            server,
            snapshot,
            "temp_c",
            Unit::Celsius,
            &p,
            Some(SECONDARY_PLOT_HEIGHT),
        );
        processes(ui, snapshot);
        None
    }
}

fn gpu_line(ui: &mut Ui, gpu: &Gpu) {
    let p = Palette::current(ui.ctx());
    ui.horizontal_wrapped(|ui| {
        ui.label(RichText::new(format!("{} {}", gpu.index, gpu.name)).strong());
        ui.label(RichText::new(gpu.vendor.label()).color(p.text_muted));
        if let Some(temperature) = gpu.temperature_c {
            ui.monospace(format!("{temperature:.0}°C"));
        }
        if let Some(power) = gpu.power_w {
            let limit = gpu
                .power_limit_w
                .map(|l| format!(" / {l:.0} W"))
                .unwrap_or_default();
            ui.monospace(format!("{power:.0}{limit} W"));
        }
    });
    let memory = format!(
        "{} / {}",
        format::bytes(gpu.memory_used),
        format::bytes(gpu.memory_total)
    );
    meter(
        ui,
        text::GPU_UTIL,
        gpu.utilization_pct,
        &format!("{:.0}%", gpu.utilization_pct),
    );
    meter(ui, text::GPU_MEMORY, gpu.memory_pct(), &memory);
}

fn plot(
    ui: &mut Ui,
    server: &ServerState,
    snapshot: &GpuSnapshot,
    metric: &str,
    unit: Unit,
    p: &Palette,
    height: Option<f32>,
) {
    let id = format!("gpu-{metric}");
    let mut plot = TimeSeriesPlot::new(&id, unit);
    let mut has_series = false;
    let labels: Vec<String> = snapshot
        .gpus
        .iter()
        .map(|g| format!("{} {}", g.index, g.name))
        .collect();
    for (index, gpu) in snapshot.gpus.iter().enumerate() {
        if let Some(series) = server.series.get(&gpu::gpu_key(gpu.index, metric)) {
            plot = plot.series(&labels[index], series, p.chart[index % p.chart.len()]);
            has_series = true;
        }
    }
    if !has_series {
        return;
    }
    if let Some(height) = height {
        plot = plot.height(height);
    }
    plot.show(ui);
    ui.add_space(GAP);
}

fn processes(ui: &mut Ui, snapshot: &GpuSnapshot) {
    let p = Palette::current(ui.ctx());
    let rows: Vec<(&Gpu, &gpu::GpuProcess)> = snapshot
        .gpus
        .iter()
        .flat_map(|g| g.processes.iter().map(move |proc| (g, proc)))
        .collect();
    if rows.is_empty() {
        return;
    }
    ui.label(
        RichText::new(text::GPU_PROCESSES.to_uppercase())
            .small()
            .color(p.text_secondary),
    );
    let columns = ["GPU", "PID", text::COL_NAME, text::GPU_MEMORY];
    Table::new("gpu-processes", &columns).show(ui, |ui| {
        for (gpu, process) in rows {
            ui.monospace(gpu.index.to_string());
            ui.monospace(process.pid.to_string());
            ui.label(&process.name);
            ui.monospace(format::bytes(process.memory_bytes));
            ui.end_row();
        }
    });
}
