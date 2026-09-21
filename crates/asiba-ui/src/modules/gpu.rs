use asiba_core::{ModuleId, ServerState};
use asiba_modules::gpu::{self, Gpu, GpuSnapshot};
use egui::{RichText, Ui};

use super::{ModuleView, Tab, ViewAction, ViewShared};
use crate::components::{Sort, SortColumn, SortKey, Table, TimeSeriesPlot, Unit, meter, sort_rows};
use crate::format;
use crate::text;
use crate::theme::{GAP, Palette, SECONDARY_PLOT_HEIGHT};

const PROCESS_SORTABLE: [SortColumn; 4] = [
    SortColumn::text(0),
    SortColumn::text(1),
    SortColumn::text(2),
    SortColumn::number(3),
];
const PROCESS_DEFAULT_SORT: Sort = Sort::descending(3);

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

    fn summary(&self, ui: &mut Ui, server: &ServerState, _shared: &ViewShared) {
        let Some(snapshot) = server.data::<GpuSnapshot>(gpu::ID) else {
            return;
        };
        for gpu in &snapshot.gpus {
            gpu_line(ui, gpu);
        }
    }

    fn page(&self, ui: &mut Ui, server: &ServerState, _shared: &ViewShared) -> Option<ViewAction> {
        let p = Palette::current(ui.ctx());
        let snapshot = server.data::<GpuSnapshot>(gpu::ID)?;
        for gpu in &snapshot.gpus {
            gpu_line(ui, gpu);
        }
        ui.add_space(GAP);
        let plots = [
            ("util_pct", text::PLOT_GPU_UTIL, Unit::Percent, None),
            (
                "mem_pct",
                text::PLOT_GPU_MEM,
                Unit::Percent,
                Some(SECONDARY_PLOT_HEIGHT),
            ),
            (
                "temp_c",
                text::PLOT_GPU_TEMP,
                Unit::Celsius,
                Some(SECONDARY_PLOT_HEIGHT),
            ),
        ];
        for (metric, title, unit, height) in plots {
            let spec = PlotSpec {
                metric,
                title,
                unit,
                height,
            };
            plot(ui, server, snapshot, &spec, &p);
        }
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

struct PlotSpec {
    metric: &'static str,
    title: &'static str,
    unit: Unit,
    height: Option<f32>,
}

fn plot(ui: &mut Ui, server: &ServerState, snapshot: &GpuSnapshot, spec: &PlotSpec, p: &Palette) {
    let (metric, unit, height) = (spec.metric, spec.unit, spec.height);
    let id = format!("gpu-{metric}");
    let mut plot = TimeSeriesPlot::new(&id, unit).title(spec.title);
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
    let table =
        Table::new("gpu-processes", &columns).sortable(&PROCESS_SORTABLE, PROCESS_DEFAULT_SORT);
    table.show_sorted(ui, |ui, sort| {
        let mut rows = rows;
        sort_rows(&mut rows, sort, |(gpu, process), column| match column {
            0 => SortKey::number(gpu.index),
            1 => SortKey::number(process.pid),
            2 => SortKey::text(&process.name),
            _ => SortKey::number(process.memory_bytes as f64),
        });
        for (gpu, process) in rows {
            ui.monospace(gpu.index.to_string());
            ui.monospace(process.pid.to_string());
            ui.label(&process.name);
            ui.monospace(format::bytes(process.memory_bytes));
            ui.end_row();
        }
    });
}
