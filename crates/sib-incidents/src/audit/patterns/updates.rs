use sib_modules::system::{self, SystemInfo};
use sib_modules::updates::{self, UpdatesSnapshot};

use crate::audit::context::AuditContext;
use crate::audit::evidence::{EvidenceSource, TAB_SUMMARY};
use crate::audit::pattern::{Area, Pattern, Verdict, Weight};

const UPTIME_WARN_DAYS: i64 = 180;
const UPTIME_FAIL_DAYS: i64 = 365;
const PENDING_WARN: u32 = 50;
const NO_DATA: &str = "the module has not collected data";

const ADVICE_UPTIME: &str = "Plan a reboot. A long uptime means the kernel has not been updated.";
const ADVICE_SECURITY_UPDATES: &str =
    "Install the security updates (apt upgrade, dnf upgrade --security).";
const ADVICE_PENDING: &str = "Update the system in the next maintenance window.";
const ADVICE_REBOOT: &str = "A new kernel or libc only takes effect after a reboot.";

pub static PATTERNS: &[Pattern] = &[
    Pattern {
        id: "updates.security",
        area: Area::Updates,
        subject: "Security updates",
        description: "Packages with security fixes that are not installed yet.",
        weight: Weight::High,
        advice: ADVICE_SECURITY_UPDATES,
        evidence: EvidenceSource::Tab(TAB_SUMMARY),
        evaluate: security_updates,
    },
    Pattern {
        id: "updates.pending",
        area: Area::Updates,
        subject: "Pending updates",
        description: "How many packages have a newer version available.",
        weight: Weight::Low,
        advice: ADVICE_PENDING,
        evidence: EvidenceSource::Tab(TAB_SUMMARY),
        evaluate: pending,
    },
    Pattern {
        id: "updates.reboot",
        area: Area::Updates,
        subject: "Reboot after updates",
        description: "The system reports that the updated kernel or libraries need a reboot.",
        weight: Weight::Medium,
        advice: ADVICE_REBOOT,
        evidence: EvidenceSource::Tab(TAB_SUMMARY),
        evaluate: reboot,
    },
    Pattern {
        id: "updates.uptime",
        area: Area::Updates,
        subject: "Uptime",
        description: "An uptime over six months means the kernel has not been restarted after updates.",
        weight: Weight::Low,
        advice: ADVICE_UPTIME,
        evidence: EvidenceSource::Tab(TAB_SUMMARY),
        evaluate: uptime,
    },
];

fn security_updates(ctx: &AuditContext<'_>) -> Vec<Verdict> {
    let Some(snapshot) = ctx.data::<UpdatesSnapshot>(updates::ID) else {
        return Verdict::skipped(NO_DATA).single();
    };
    Verdict::graded(
        snapshot.security > 0,
        false,
        format!("{} waiting to be installed", snapshot.security),
    )
    .single()
}

fn pending(ctx: &AuditContext<'_>) -> Vec<Verdict> {
    let Some(snapshot) = ctx.data::<UpdatesSnapshot>(updates::ID) else {
        return Verdict::skipped(NO_DATA).single();
    };
    Verdict::graded(
        false,
        snapshot.pending >= PENDING_WARN,
        format!(
            "{} packages ({})",
            snapshot.pending,
            snapshot.manager.label()
        ),
    )
    .single()
}

fn reboot(ctx: &AuditContext<'_>) -> Vec<Verdict> {
    let Some(snapshot) = ctx.data::<UpdatesSnapshot>(updates::ID) else {
        return Verdict::skipped(NO_DATA).single();
    };
    if snapshot.reboot_required {
        return Verdict::warn("needed").single();
    }
    Verdict::pass("not needed").single()
}

fn uptime(ctx: &AuditContext<'_>) -> Vec<Verdict> {
    let Some(info) = ctx.data::<SystemInfo>(system::ID) else {
        return Verdict::skipped(NO_DATA).single();
    };
    let days = info.uptime.as_secs() as i64 / 86_400;
    Verdict::graded(
        days >= UPTIME_FAIL_DAYS,
        days >= UPTIME_WARN_DAYS,
        format!("{days} days"),
    )
    .single()
}
