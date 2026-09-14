use asiba_core::{ModuleId, ServerState, Snapshot};
use asiba_modules::system::{self, SystemInfo};
use egui::{Grid, RichText, Ui};

use super::{ModuleView, Tab};
use crate::format;
use crate::text;
use crate::theme::Palette;

pub struct SystemView;

impl ModuleView for SystemView {
    fn id(&self) -> ModuleId {
        system::ID
    }

    fn title(&self) -> &'static str {
        text::MODULE_SYSTEM
    }

    fn tab(&self) -> Tab {
        Tab::Summary
    }

    fn summary(&self, ui: &mut Ui, server: &ServerState) {
        if let Some(info) = server.data::<SystemInfo>(system::ID) {
            grid(ui, "system-summary", info);
        }
    }

    fn preview(&self, ui: &mut Ui, snapshot: &Snapshot) {
        if let Some(info) = snapshot.downcast::<SystemInfo>() {
            grid(ui, "system-preview", info);
        }
    }
}

fn grid(ui: &mut Ui, id: &str, info: &SystemInfo) {
    let p = Palette::current(ui.ctx());
    Grid::new(id)
        .num_columns(2)
        .spacing([16.0, 4.0])
        .show(ui, |ui| {
            for (key, value) in rows(info) {
                ui.label(RichText::new(key).color(p.text_secondary));
                ui.monospace(value);
                ui.end_row();
            }
        });
}

fn rows(info: &SystemInfo) -> Vec<(&'static str, String)> {
    let load = format!(
        "{:.2}  {:.2}  {:.2}",
        info.load.one, info.load.five, info.load.fifteen
    );
    let cpu = format!("{} × {}", info.cpu_cores, info.cpu_model);
    let mut rows = vec![
        (text::SYS_HOSTNAME, info.hostname.clone()),
        (text::SYS_OS, info.os_name.clone()),
        (text::SYS_KERNEL, format!("{} ({})", info.kernel, info.arch)),
        (text::SYS_UPTIME, info.uptime_human()),
        (text::SYS_LOAD, load),
        (text::SYS_CPU, cpu),
        (text::SYS_MEMORY, format::bytes(info.mem_total_bytes)),
    ];
    if info.swap_total_bytes > 0 {
        rows.push((text::SYS_SWAP, format::bytes(info.swap_total_bytes)));
    }
    if let Some(virt) = &info.virtualization {
        rows.push((text::SYS_VIRT, virt.clone()));
    }
    if let Some(tz) = &info.timezone {
        rows.push((text::SYS_TIMEZONE, tz.clone()));
    }
    if info.clock_offset_secs.abs() > 1 {
        rows.push((
            text::SYS_CLOCK,
            format::signed_seconds(info.clock_offset_secs),
        ));
    }
    rows
}
