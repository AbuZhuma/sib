use asiba_core::{ModuleId, ServerState};
use asiba_modules::disk::{self, DiskSnapshot};
use egui::{RichText, Ui};

use super::{ModuleView, Tab};
use crate::components::{Table, TimeSeriesPlot, Unit, meter};
use crate::format;
use crate::text;
use crate::theme::{GAP, Palette};

const SUMMARY_FILESYSTEMS: usize = 4;

pub struct DiskView;

impl ModuleView for DiskView {
    fn id(&self) -> ModuleId {
        disk::ID
    }

    fn title(&self) -> &'static str {
        text::MODULE_DISK
    }

    fn tab(&self) -> Tab {
        Tab::Resources
    }

    fn summary(&self, ui: &mut Ui, server: &ServerState) {
        let Some(snapshot) = server.data::<DiskSnapshot>(disk::ID) else {
            return;
        };
        for fs in snapshot.filesystems.iter().take(SUMMARY_FILESYSTEMS) {
            let detail = format!(
                "{} / {}",
                format::bytes(fs.used_bytes),
                format::bytes(fs.total_bytes)
            );
            meter(ui, &fs.mount, fs.used_pct(), &detail);
        }
        io_line(ui, server);
    }

    fn page(&self, ui: &mut Ui, server: &ServerState) {
        let p = Palette::current(ui.ctx());
        let Some(snapshot) = server.data::<DiskSnapshot>(disk::ID) else {
            return;
        };
        filesystems_table(ui, snapshot);
        ui.add_space(GAP);
        let mut plot = TimeSeriesPlot::new("disk-io-plot", Unit::BytesPerSecond).height(160.0);
        if let Some(series) = server.series.get(disk::KEY_READ_BPS) {
            plot = plot.series(text::DISK_READ, series, p.chart[0]);
        }
        if let Some(series) = server.series.get(disk::KEY_WRITE_BPS) {
            plot = plot.series(text::DISK_WRITE, series, p.chart[2]);
        }
        plot.show(ui);
        ui.add_space(GAP);
        devices_table(ui, snapshot);
    }
}

fn io_line(ui: &mut Ui, server: &ServerState) {
    let p = Palette::current(ui.ctx());
    let read = server.latest_value(disk::KEY_READ_BPS);
    let write = server.latest_value(disk::KEY_WRITE_BPS);
    if let (Some(read), Some(write)) = (read, write) {
        ui.monospace(
            RichText::new(format!(
                "R {}  W {}",
                format::bytes_per_second(read),
                format::bytes_per_second(write)
            ))
            .color(p.text_secondary),
        );
    }
}

fn filesystems_table(ui: &mut Ui, snapshot: &DiskSnapshot) {
    let columns = [
        text::DISK_MOUNT,
        text::DISK_DEVICE,
        text::DISK_USED,
        text::DISK_TOTAL,
        text::DISK_AVAILABLE,
        text::DISK_INODES,
    ];
    Table::new("disk-filesystems", &columns).show(ui, |ui| {
        for fs in &snapshot.filesystems {
            ui.monospace(&fs.mount);
            ui.monospace(&fs.device);
            ui.monospace(format!(
                "{} ({:.0}%)",
                format::bytes(fs.used_bytes),
                fs.used_pct()
            ));
            ui.monospace(format::bytes(fs.total_bytes));
            ui.monospace(format::bytes(fs.available_bytes));
            let inodes = if fs.inodes_total > 0 {
                format!("{:.0}%", fs.inodes_used_pct())
            } else {
                "—".to_owned()
            };
            ui.monospace(inodes);
            ui.end_row();
        }
    });
}

fn devices_table(ui: &mut Ui, snapshot: &DiskSnapshot) {
    let columns = [
        text::DISK_DEVICE,
        text::DISK_READ,
        text::DISK_WRITE,
        text::DISK_IOPS,
        text::DISK_UTIL,
    ];
    Table::new("disk-devices", &columns).show(ui, |ui| {
        for device in &snapshot.devices {
            ui.monospace(&device.name);
            match &device.rates {
                Some(rates) => {
                    ui.monospace(format::bytes_per_second(rates.read_bps));
                    ui.monospace(format::bytes_per_second(rates.write_bps));
                    ui.monospace(format!("{:.0} / {:.0}", rates.read_iops, rates.write_iops));
                    ui.monospace(format!("{:.0}%", rates.util_pct));
                }
                None => {
                    for _ in 0..4 {
                        ui.monospace("—");
                    }
                }
            }
            ui.end_row();
        }
    });
}
