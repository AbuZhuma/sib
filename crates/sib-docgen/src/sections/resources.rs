use sib_modules::disk::{self, DiskSnapshot};
use sib_modules::memory::{self, MemorySnapshot};
use sib_modules::network::{self, NetworkSnapshot};
use sib_modules::{cpu, processes};

use crate::section::{DocContext, Section, SectionId};
use crate::series_stats::{self, MetricStats};
use crate::write::{
    NONE, blank, bullet, bytes, bytes_per_second, field, heading, list, percent, subheading, table,
};

pub struct ResourcesSection;

const MAX_FILESYSTEMS: usize = 12;
const MAX_INTERFACES: usize = 8;

type Formatter = fn(f64) -> String;

const METRICS: [(&str, &str, &str, Formatter); 8] = [
    ("CPU", "cpu_total_pct", cpu::KEY_TOTAL, percent),
    ("CPU iowait", "cpu_iowait_pct", cpu::KEY_IOWAIT, percent),
    ("Memory", "memory_used_pct", memory::KEY_USED_PCT, percent),
    ("Swap", "swap_used_pct", memory::KEY_SWAP_USED_PCT, percent),
    (
        "Disk /",
        "disk_root_used_pct",
        disk::KEY_ROOT_USED_PCT,
        percent,
    ),
    (
        "Network RX",
        "network_rx",
        network::KEY_RX_BPS,
        bytes_per_second,
    ),
    (
        "Network TX",
        "network_tx",
        network::KEY_TX_BPS,
        bytes_per_second,
    ),
    ("Processes", "process_count", processes::KEY_COUNT, count),
];

fn count(value: f64) -> String {
    format!("{value:.0}")
}

fn stat_cells(stats: &MetricStats, fmt: Formatter) -> Vec<String> {
    let window = |w: Option<series_stats::WindowStats>| {
        w.map(|w| (fmt(w.average), fmt(w.max)))
            .unwrap_or_else(|| (NONE.to_owned(), NONE.to_owned()))
    };
    let (hour_avg, hour_max) = window(stats.hour);
    let (day_avg, day_max) = window(stats.day);
    vec![fmt(stats.current), hour_avg, hour_max, day_avg, day_max]
}

fn metric_rows(ctx: &DocContext<'_>, use_key: bool) -> Vec<Vec<String>> {
    METRICS
        .iter()
        .filter_map(|(label, key, metric, fmt)| {
            let stats = series_stats::metric(ctx.server, metric)?;
            let name = if use_key { *key } else { *label };
            let mut row = vec![name.to_owned()];
            row.extend(stat_cells(&stats, *fmt));
            Some(row)
        })
        .collect()
}

fn filesystem_rows(snapshot: &DiskSnapshot) -> Vec<Vec<String>> {
    snapshot
        .filesystems
        .iter()
        .take(MAX_FILESYSTEMS)
        .map(|fs| {
            vec![
                fs.mount.clone(),
                fs.device.clone(),
                percent(fs.used_pct()),
                bytes(fs.used_bytes),
                bytes(fs.total_bytes),
                percent(fs.inodes_used_pct()),
            ]
        })
        .collect()
}

fn interface_rows(snapshot: &NetworkSnapshot) -> Vec<Vec<String>> {
    snapshot
        .interfaces
        .iter()
        .filter(|i| !i.is_loopback())
        .take(MAX_INTERFACES)
        .map(|i| {
            let rates = i.rates.unwrap_or_default();
            vec![
                i.name.clone(),
                i.state.clone(),
                i.addresses.join(" "),
                bytes_per_second(rates.rx_bps),
                bytes_per_second(rates.tx_bps),
                (i.rx.errors + i.tx.errors).to_string(),
                (i.rx.drops + i.tx.drops).to_string(),
            ]
        })
        .collect()
}

fn memory_rows(snapshot: &MemorySnapshot) -> Vec<(&'static str, &'static str, String)> {
    let mut rows = vec![
        ("Used", "used", bytes(snapshot.used_bytes())),
        ("Available", "available", bytes(snapshot.available_bytes)),
        ("Cache", "cached", bytes(snapshot.cached_bytes)),
        ("Swap used", "swap_used", bytes(snapshot.swap_used_bytes())),
        ("OOM kills", "oom_kills", snapshot.oom_kills.to_string()),
    ];
    if let Some(pressure) = snapshot.pressure {
        rows.push((
            "Pressure (some avg10)",
            "pressure_some_avg10",
            format!("{:.1}", pressure.some.avg10),
        ));
    }
    rows
}

fn human_memory(out: &mut String, ctx: &DocContext<'_>) {
    let Some(memory) = ctx.server.data::<MemorySnapshot>(memory::ID) else {
        return;
    };
    subheading(out, "Memory");
    for (label, _, value) in memory_rows(memory) {
        bullet(out, label, value);
    }
    blank(out);
}

fn human_filesystems(out: &mut String, ctx: &DocContext<'_>) {
    let Some(disk) = ctx.server.data::<DiskSnapshot>(disk::ID) else {
        return;
    };
    subheading(out, "File systems");
    let columns = ["Mount", "Device", "Used", "Used", "Total", "Inodes"];
    table(out, &columns, &filesystem_rows(disk));
}

fn human_interfaces(out: &mut String, ctx: &DocContext<'_>) {
    let Some(net) = ctx.server.data::<NetworkSnapshot>(network::ID) else {
        return;
    };
    subheading(out, "Interfaces");
    let columns = [
        "Interface",
        "State",
        "Addresses",
        "RX",
        "TX",
        "Errors",
        "Drops",
    ];
    table(out, &columns, &interface_rows(net));
}

impl Section for ResourcesSection {
    fn id(&self) -> SectionId {
        SectionId::Resources
    }

    fn is_available(&self, ctx: &DocContext<'_>) -> bool {
        !metric_rows(ctx, false).is_empty()
    }

    fn human(&self, out: &mut String, ctx: &DocContext<'_>) {
        heading(out, "Resources");
        table(
            out,
            &[
                "Metric", "Now", "Avg 1 h", "Max 1 h", "Avg 24 h", "Max 24 h",
            ],
            &metric_rows(ctx, false),
        );
        human_memory(out, ctx);
        human_filesystems(out, ctx);
        human_interfaces(out, ctx);
    }

    fn llm(&self, out: &mut String, ctx: &DocContext<'_>) {
        heading(out, "Resources");
        field(
            out,
            "metrics",
            "name | now | avg_1h | max_1h | avg_24h | max_24h",
        );
        list(out, "", &metric_rows(ctx, true));
        if let Some(memory) = ctx.server.data::<MemorySnapshot>(memory::ID) {
            for (_, key, value) in memory_rows(memory) {
                field(out, &format!("memory_{key}"), value);
            }
            blank(out);
        }
        if let Some(disk) = ctx.server.data::<DiskSnapshot>(disk::ID) {
            field(
                out,
                "filesystems",
                "mount | device | used_pct | used | total | inodes_pct",
            );
            list(out, "", &filesystem_rows(disk));
        }
        if let Some(net) = ctx.server.data::<NetworkSnapshot>(network::ID) {
            field(
                out,
                "interfaces",
                "name | state | addresses | rx | tx | errors | drops",
            );
            list(out, "", &interface_rows(net));
        }
    }
}
