use asiba_modules::system::{self, SystemInfo};
use asiba_modules::updates::{self, UpdatesSnapshot};

use crate::audit::context::AuditContext;
use crate::audit::evidence::{EvidenceSource, TAB_SUMMARY};
use crate::audit::pattern::{Area, Pattern, Verdict, Weight};

const UPTIME_WARN_DAYS: i64 = 180;
const UPTIME_FAIL_DAYS: i64 = 365;
const PENDING_WARN: u32 = 50;
const NO_DATA: &str = "модуль не собрал данные";

const ADVICE_UPTIME: &str =
    "Запланируйте перезагрузку: длительный аптайм означает, что ядро не обновлялось.";
const ADVICE_SECURITY_UPDATES: &str =
    "Установите обновления безопасности (apt upgrade / dnf upgrade --security).";
const ADVICE_PENDING: &str = "Обновите систему в ближайшее окно обслуживания.";
const ADVICE_REBOOT: &str = "Обновлённое ядро или libc заработают только после перезагрузки.";

pub static PATTERNS: &[Pattern] = &[
    Pattern {
        id: "updates.security",
        area: Area::Updates,
        subject: "Обновления безопасности",
        description: "Пакеты с исправлениями уязвимостей, которые ещё не установлены.",
        weight: Weight::High,
        advice: ADVICE_SECURITY_UPDATES,
        evidence: EvidenceSource::Tab(TAB_SUMMARY),
        evaluate: security_updates,
    },
    Pattern {
        id: "updates.pending",
        area: Area::Updates,
        subject: "Ожидающие обновления",
        description: "Общее число пакетов, для которых доступна новая версия.",
        weight: Weight::Low,
        advice: ADVICE_PENDING,
        evidence: EvidenceSource::Tab(TAB_SUMMARY),
        evaluate: pending,
    },
    Pattern {
        id: "updates.reboot",
        area: Area::Updates,
        subject: "Перезагрузка после обновлений",
        description: "Система сообщает, что обновлённые ядро или библиотеки требуют перезагрузки.",
        weight: Weight::Medium,
        advice: ADVICE_REBOOT,
        evidence: EvidenceSource::Tab(TAB_SUMMARY),
        evaluate: reboot,
    },
    Pattern {
        id: "updates.uptime",
        area: Area::Updates,
        subject: "Аптайм",
        description: "Аптайм больше полугода означает, что ядро не перезагружалось после обновлений.",
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
        format!("{} ожидают установки", snapshot.security),
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
            "{} пакетов ({})",
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
        return Verdict::warn("требуется").single();
    }
    Verdict::pass("не требуется").single()
}

fn uptime(ctx: &AuditContext<'_>) -> Vec<Verdict> {
    let Some(info) = ctx.data::<SystemInfo>(system::ID) else {
        return Verdict::skipped(NO_DATA).single();
    };
    let days = info.uptime.as_secs() as i64 / 86_400;
    Verdict::graded(
        days >= UPTIME_FAIL_DAYS,
        days >= UPTIME_WARN_DAYS,
        format!("{days} дней"),
    )
    .single()
}
