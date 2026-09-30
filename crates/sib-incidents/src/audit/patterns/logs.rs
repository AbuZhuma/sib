use chrono::Duration;
use sib_modules::logs::{self, LogsSnapshot};
use sib_modules::system::{self, SystemInfo};

use crate::audit::context::AuditContext;
use crate::audit::evidence::{EvidenceSource, TAB_LOGS, TAB_SUMMARY};
use crate::audit::pattern::{Area, Pattern, Verdict, Weight};

const CLOCK_WARN_SECS: i64 = 5;
const CLOCK_FAIL_SECS: i64 = 60;
const ERRORS_HOUR_WARN: usize = 10;
const ERRORS_HOUR_FAIL: usize = 100;
const CRITICAL_PRIORITY: u8 = 2;
const ERROR_PRIORITY: u8 = 3;
const NO_DATA: &str = "the module has not collected data";

const ADVICE_ERRORS: &str = "Open the Logs tab: repeats are grouped by source.";
const ADVICE_CRITICAL: &str = "Entries at crit, alert and emerg level come from hardware, the file system or the kernel. Look at each one.";
const ADVICE_CLOCK: &str =
    "Turn on time sync: timedatectl set-ntp true, then check chrony or systemd-timesyncd.";

pub static PATTERNS: &[Pattern] = &[
    Pattern {
        id: "logs.errors_hour",
        area: Area::Logs,
        subject: "Journal errors in the last hour",
        description: "Number of entries at err level and above in the last hour.",
        weight: Weight::Medium,
        advice: ADVICE_ERRORS,
        evidence: EvidenceSource::Tab(TAB_LOGS),
        evaluate: errors_hour,
    },
    Pattern {
        id: "logs.critical_day",
        area: Area::Logs,
        subject: "Critical entries in the last day",
        description: "Entries at crit, alert and emerg level in the last day.",
        weight: Weight::High,
        advice: ADVICE_CRITICAL,
        evidence: EvidenceSource::Tab(TAB_LOGS),
        evaluate: critical_day,
    },
    Pattern {
        id: "logs.clock",
        area: Area::Logs,
        subject: "Server clock",
        description: "How far the server clock is from this machine. With a large gap, journals from different servers no longer line up.",
        weight: Weight::Low,
        advice: ADVICE_CLOCK,
        evidence: EvidenceSource::Tab(TAB_SUMMARY),
        evaluate: clock,
    },
];

fn errors_hour(ctx: &AuditContext<'_>) -> Vec<Verdict> {
    let Some(snapshot) = ctx.data::<LogsSnapshot>(logs::ID) else {
        return Verdict::skipped(NO_DATA).single();
    };
    let errors = snapshot.count_since(Duration::hours(1), ERROR_PRIORITY);
    Verdict::graded(
        errors >= ERRORS_HOUR_FAIL,
        errors >= ERRORS_HOUR_WARN,
        format!("{errors} entries at err level and above"),
    )
    .single()
}

fn critical_day(ctx: &AuditContext<'_>) -> Vec<Verdict> {
    let Some(snapshot) = ctx.data::<LogsSnapshot>(logs::ID) else {
        return Verdict::skipped(NO_DATA).single();
    };
    let critical = snapshot.count_since(Duration::hours(24), CRITICAL_PRIORITY);
    Verdict::graded(
        critical > 0,
        false,
        format!("{critical} entries at crit level and above"),
    )
    .single()
}

fn clock(ctx: &AuditContext<'_>) -> Vec<Verdict> {
    let Some(info) = ctx.data::<SystemInfo>(system::ID) else {
        return Verdict::skipped(NO_DATA).single();
    };
    let offset = info.clock_offset_secs.abs();
    Verdict::graded(
        offset >= CLOCK_FAIL_SECS,
        offset >= CLOCK_WARN_SECS,
        format!("{} s off this machine", info.clock_offset_secs),
    )
    .single()
}
