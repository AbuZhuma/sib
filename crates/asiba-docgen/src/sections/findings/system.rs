use asiba_core::Severity;
use asiba_modules::disk::{self, DiskSnapshot};
use asiba_modules::memory::{self, MemorySnapshot};
use asiba_modules::system::{self, SystemInfo};
use asiba_modules::updates::{self, UpdatesSnapshot};

use super::Finding;
use crate::section::DocContext;
use crate::write::bytes;

const DISK_CRITICAL_PCT: f64 = 90.0;
const DISK_WARNING_PCT: f64 = 80.0;
const INODES_WARNING_PCT: f64 = 90.0;
const SWAP_WARNING_PCT: f64 = 80.0;
const CLOCK_OFFSET_WARNING_SECS: i64 = 60;

pub fn collect(ctx: &DocContext<'_>, out: &mut Vec<Finding>) {
    connection(ctx, out);
    modules(ctx, out);
    clock(ctx, out);
    disks(ctx, out);
    memory(ctx, out);
    updates(ctx, out);
}

fn connection(ctx: &DocContext<'_>, out: &mut Vec<Finding>) {
    if !ctx.server.connection.is_online() {
        out.push(Finding::new(
            Severity::Critical,
            "connection",
            "server is offline or unreachable",
        ));
    }
}

fn modules(ctx: &DocContext<'_>, out: &mut Vec<Finding>) {
    for (id, state) in &ctx.server.modules {
        if let Some(error) = &state.last_error {
            out.push(Finding::new(
                Severity::Warning,
                "collection",
                format!("module {} failed: {error}", id.0),
            ));
        }
    }
}

fn clock(ctx: &DocContext<'_>, out: &mut Vec<Finding>) {
    let Some(info) = ctx.server.data::<SystemInfo>(system::ID) else {
        return;
    };
    if info.clock_offset_secs.abs() > CLOCK_OFFSET_WARNING_SECS {
        out.push(Finding::new(
            Severity::Warning,
            "system",
            format!("clock differs from local by {} s", info.clock_offset_secs),
        ));
    }
}

fn disks(ctx: &DocContext<'_>, out: &mut Vec<Finding>) {
    let Some(snapshot) = ctx.server.data::<DiskSnapshot>(disk::ID) else {
        return;
    };
    for fs in &snapshot.filesystems {
        let used = fs.used_pct();
        let severity = if used >= DISK_CRITICAL_PCT {
            Severity::Critical
        } else if used >= DISK_WARNING_PCT {
            Severity::Warning
        } else {
            continue;
        };
        out.push(Finding::new(
            severity,
            "disk",
            format!(
                "{} is {used:.0}% full ({} free)",
                fs.mount,
                bytes(fs.available_bytes)
            ),
        ));
    }
    for fs in &snapshot.filesystems {
        if fs.inodes_used_pct() >= INODES_WARNING_PCT {
            out.push(Finding::new(
                Severity::Warning,
                "disk",
                format!("{} inodes {:.0}% used", fs.mount, fs.inodes_used_pct()),
            ));
        }
    }
}

fn memory(ctx: &DocContext<'_>, out: &mut Vec<Finding>) {
    let Some(snapshot) = ctx.server.data::<MemorySnapshot>(memory::ID) else {
        return;
    };
    if snapshot.oom_kills > 0 {
        out.push(Finding::new(
            Severity::Warning,
            "memory",
            format!(
                "kernel OOM killer fired {} times since boot",
                snapshot.oom_kills
            ),
        ));
    }
    if snapshot.swap_total_bytes > 0 && snapshot.swap_used_pct() >= SWAP_WARNING_PCT {
        out.push(Finding::new(
            Severity::Warning,
            "memory",
            format!("swap is {:.0}% used", snapshot.swap_used_pct()),
        ));
    }
}

fn updates(ctx: &DocContext<'_>, out: &mut Vec<Finding>) {
    let Some(snapshot) = ctx.server.data::<UpdatesSnapshot>(updates::ID) else {
        return;
    };
    if snapshot.security > 0 {
        out.push(Finding::new(
            Severity::Warning,
            "updates",
            format!("{} security updates pending", snapshot.security),
        ));
    }
    if snapshot.reboot_required {
        out.push(Finding::new(
            Severity::Info,
            "updates",
            "reboot required to apply updates",
        ));
    }
}
