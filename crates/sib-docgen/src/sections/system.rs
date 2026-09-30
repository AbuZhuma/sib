use sib_modules::system::{self, SystemInfo};

use crate::section::{DocContext, Section, SectionId};
use crate::write::{blank, bullet, bytes, field, heading};

pub struct SystemSection;

fn info<'a>(ctx: &'a DocContext<'_>) -> Option<&'a SystemInfo> {
    ctx.server.data::<SystemInfo>(system::ID)
}

fn rows(info: &SystemInfo) -> Vec<(&'static str, &'static str, String)> {
    let mut rows = vec![
        ("Hostname", "hostname", info.hostname.clone()),
        ("ОС", "os", info.os_name.clone()),
        ("Ядро", "kernel", format!("{} ({})", info.kernel, info.arch)),
        (
            "CPU",
            "cpu",
            format!("{} × {}", info.cpu_cores, info.cpu_model),
        ),
        ("RAM", "ram", bytes(info.mem_total_bytes)),
        ("Swap", "swap", bytes(info.swap_total_bytes)),
        ("Uptime", "uptime", info.uptime_human()),
        (
            "Load average",
            "load_average",
            format!(
                "{:.2} {:.2} {:.2}",
                info.load.one, info.load.five, info.load.fifteen
            ),
        ),
    ];
    if let Some(virt) = &info.virtualization {
        rows.push(("Виртуализация", "virtualization", virt.clone()));
    }
    if let Some(timezone) = &info.timezone {
        rows.push(("Часовой пояс", "timezone", timezone.clone()));
    }
    if info.clock_offset_secs.abs() > 1 {
        rows.push((
            "Расхождение часов",
            "clock_offset_seconds",
            info.clock_offset_secs.to_string(),
        ));
    }
    rows
}

impl Section for SystemSection {
    fn id(&self) -> SectionId {
        SectionId::System
    }

    fn is_available(&self, ctx: &DocContext<'_>) -> bool {
        info(ctx).is_some()
    }

    fn human(&self, out: &mut String, ctx: &DocContext<'_>) {
        let Some(info) = info(ctx) else {
            return;
        };
        heading(out, "Система");
        for (label, _, value) in rows(info) {
            bullet(out, label, value);
        }
        blank(out);
    }

    fn llm(&self, out: &mut String, ctx: &DocContext<'_>) {
        let Some(info) = info(ctx) else {
            return;
        };
        heading(out, "System");
        for (_, key, value) in rows(info) {
            field(out, key, value);
        }
        blank(out);
    }
}
