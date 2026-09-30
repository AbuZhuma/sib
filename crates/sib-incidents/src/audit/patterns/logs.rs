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
const NO_DATA: &str = "модуль не собрал данные";

const ADVICE_ERRORS: &str = "Откройте вкладку «Логи»: повторы сгруппированы по источнику.";
const ADVICE_CRITICAL: &str = "Записи уровня crit, alert и emerg относятся к оборудованию, файловой системе или ядру. Разберите каждую.";
const ADVICE_CLOCK: &str = "Включите синхронизацию времени: timedatectl set-ntp true, затем проверьте chrony или systemd-timesyncd.";

pub static PATTERNS: &[Pattern] = &[
    Pattern {
        id: "logs.errors_hour",
        area: Area::Logs,
        subject: "Ошибки в журнале за час",
        description: "Число записей уровня err и выше за последний час.",
        weight: Weight::Medium,
        advice: ADVICE_ERRORS,
        evidence: EvidenceSource::Tab(TAB_LOGS),
        evaluate: errors_hour,
    },
    Pattern {
        id: "logs.critical_day",
        area: Area::Logs,
        subject: "Критичные записи за сутки",
        description: "Записи уровня crit, alert и emerg за последние сутки.",
        weight: Weight::High,
        advice: ADVICE_CRITICAL,
        evidence: EvidenceSource::Tab(TAB_LOGS),
        evaluate: critical_day,
    },
    Pattern {
        id: "logs.clock",
        area: Area::Logs,
        subject: "Часы сервера",
        description: "Расхождение часов сервера с локальной машиной: при большом расхождении журналы разных серверов не сопоставляются.",
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
        format!("{errors} записей уровня err и выше"),
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
        format!("{critical} записей уровня crit и выше"),
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
        format!("расхождение с этой машиной {} с", info.clock_offset_secs),
    )
    .single()
}
