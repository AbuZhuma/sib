use asiba_core::{ModuleId, ServerState};
use asiba_modules::disk::{self, DeviceIo, DiskSnapshot, Filesystem};
use egui::{RichText, Ui};

use super::{ModuleView, Tab, ViewAction, ViewShared};
use crate::components::{Sort, SortColumn, SortKey, Table, TimeSeriesPlot, Unit, meter, sort_rows};
use crate::format;
use crate::text;
use crate::theme::{GAP, Palette};

const FILESYSTEM_SORTABLE: [SortColumn; 5] = [
    SortColumn::text(0),
    SortColumn::number(2),
    SortColumn::number(3),
    SortColumn::number(4),
    SortColumn::number(5),
];
const FILESYSTEM_DEFAULT_SORT: Sort = Sort::descending(2);
const DEVICE_SORTABLE: [SortColumn; 5] = [
    SortColumn::text(0),
    SortColumn::number(1),
    SortColumn::number(2),
    SortColumn::number(3),
    SortColumn::number(4),
];
const DEVICE_DEFAULT_SORT: Sort = Sort::ascending(0);

const SUMMARY_FILESYSTEMS: usize = 6;

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

    fn summary(&self, ui: &mut Ui, server: &ServerState, _shared: &ViewShared) {
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

    fn page(&self, ui: &mut Ui, server: &ServerState, _shared: &ViewShared) -> Option<ViewAction> {
        let p = Palette::current(ui.ctx());
        let snapshot = server.data::<DiskSnapshot>(disk::ID)?;
        filesystems_table(ui, snapshot);
        ui.add_space(GAP);
        let mut plot =
            TimeSeriesPlot::new("disk-io-plot", Unit::BytesPerSecond).title(text::PLOT_DISK_IO);
        if let Some(series) = server.series.get(disk::KEY_READ_BPS) {
            plot = plot.series(text::DISK_READ, series, p.chart[0]);
        }
        if let Some(series) = server.series.get(disk::KEY_WRITE_BPS) {
            plot = plot.series(text::DISK_WRITE, series, p.chart[2]);
        }
        plot.show(ui);
        ui.add_space(GAP);
        devices_table(ui, snapshot);
        None
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
    let table = Table::new("disk-filesystems", &columns)
        .sortable(&FILESYSTEM_SORTABLE, FILESYSTEM_DEFAULT_SORT);
    table.show_sorted(ui, |ui, sort| {
        let mut rows: Vec<&Filesystem> = snapshot.filesystems.iter().collect();
        sort_rows(&mut rows, sort, |fs, column| filesystem_key(fs, column));
        for fs in rows {
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
                "-".to_owned()
            };
            ui.monospace(inodes);
            ui.end_row();
        }
    });
}

fn filesystem_key(fs: &Filesystem, column: usize) -> SortKey {
    match column {
        0 => SortKey::text(&fs.mount),
        2 => SortKey::number(fs.used_pct()),
        3 => SortKey::number(fs.total_bytes as f64),
        4 => SortKey::number(fs.available_bytes as f64),
        _ => SortKey::number(fs.inodes_used_pct()),
    }
}

fn device_key(device: &DeviceIo, column: usize) -> SortKey {
    let rates = device.rates.as_ref();
    match column {
        0 => SortKey::text(&device.name),
        1 => SortKey::optional(rates.map(|r| r.read_bps)),
        2 => SortKey::optional(rates.map(|r| r.write_bps)),
        3 => SortKey::optional(rates.map(|r| r.read_iops + r.write_iops)),
        _ => SortKey::optional(rates.map(|r| r.util_pct)),
    }
}

fn devices_table(ui: &mut Ui, snapshot: &DiskSnapshot) {
    let columns = [
        text::DISK_DEVICE,
        text::DISK_READ,
        text::DISK_WRITE,
        text::DISK_IOPS,
        text::DISK_UTIL,
    ];
    let table =
        Table::new("disk-devices", &columns).sortable(&DEVICE_SORTABLE, DEVICE_DEFAULT_SORT);
    table.show_sorted(ui, |ui, sort| {
        let mut rows: Vec<&DeviceIo> = snapshot.devices.iter().collect();
        sort_rows(&mut rows, sort, |device, column| device_key(device, column));
        for device in rows {
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
                        ui.monospace("-");
                    }
                }
            }
            ui.end_row();
        }
    });
}
