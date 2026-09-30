use sib_modules::gpu::{self, Gpu, GpuSnapshot};

use crate::section::{DocContext, Section, SectionId};
use crate::write::{NONE, blank, bytes, field, heading, list, percent, table};

pub struct GpuSection;

fn snapshot<'a>(ctx: &'a DocContext<'_>) -> Option<&'a GpuSnapshot> {
    ctx.server.data::<GpuSnapshot>(gpu::ID)
}

fn row(gpu: &Gpu) -> Vec<String> {
    vec![
        gpu.index.to_string(),
        gpu.name.clone(),
        percent(gpu.utilization_pct),
        format!("{} / {}", bytes(gpu.memory_used), bytes(gpu.memory_total)),
        gpu.temperature_c
            .map(|t| format!("{t:.0}°C"))
            .unwrap_or_else(|| NONE.to_owned()),
        gpu.power_w
            .map(|p| format!("{p:.0} W"))
            .unwrap_or_else(|| NONE.to_owned()),
        gpu.processes.len().to_string(),
    ]
}

fn rows(snapshot: &GpuSnapshot) -> Vec<Vec<String>> {
    snapshot.gpus.iter().map(row).collect()
}

impl Section for GpuSection {
    fn id(&self) -> SectionId {
        SectionId::Gpu
    }

    fn is_available(&self, ctx: &DocContext<'_>) -> bool {
        snapshot(ctx).is_some_and(|s| !s.gpus.is_empty())
    }

    fn human(&self, out: &mut String, ctx: &DocContext<'_>) {
        let Some(snapshot) = snapshot(ctx) else {
            return;
        };
        heading(out, "GPU");
        table(
            out,
            &[
                "#",
                "Модель",
                "Загрузка",
                "Память",
                "Темп.",
                "Мощность",
                "Процессов",
            ],
            &rows(snapshot),
        );
    }

    fn llm(&self, out: &mut String, ctx: &DocContext<'_>) {
        let Some(snapshot) = snapshot(ctx) else {
            return;
        };
        heading(out, "GPU");
        field(
            out,
            "gpus",
            "index | name | utilization | memory | temperature | power | processes",
        );
        list(out, "", &rows(snapshot));
        blank(out);
    }
}
