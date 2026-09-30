use sib_modules::security::{MacStatus, SecuritySnapshot};

use super::with_security;
use crate::audit::context::AuditContext;
use crate::audit::evidence::{EvidenceSource, TAB_SUMMARY};
use crate::audit::pattern::{Area, Pattern, Verdict, Weight};

const ADVICE_MAC: &str = "Включите SELinux в режиме enforcing или AppArmor.";
const ADVICE_NTP: &str =
    "Включите синхронизацию времени: timedatectl set-ntp true (chrony или systemd-timesyncd).";
const ADVICE_AUTO_UPDATES: &str = "Включите автоматические обновления безопасности: unattended-upgrades (Debian/Ubuntu) или dnf-automatic (RHEL/Fedora).";
const ADVICE_AUDITD: &str = "Установите и включите auditd.";

pub static PATTERNS: &[Pattern] = &[
    Pattern {
        id: "hardening.mac",
        area: Area::Hardening,
        subject: "SELinux / AppArmor",
        description: "Мандатный контроль доступа ограничивает сервис его профилем: обращения за пределы профиля запрещены, даже если сервис скомпрометирован.",
        weight: Weight::Medium,
        advice: ADVICE_MAC,
        evidence: EvidenceSource::None,
        evaluate: mac,
    },
    Pattern {
        id: "hardening.ntp",
        area: Area::Hardening,
        subject: "Синхронизация времени",
        description: "Без синхронизации времени расходятся метки в журналах и нарушается проверка TLS-сертификатов и одноразовых кодов.",
        weight: Weight::Low,
        advice: ADVICE_NTP,
        evidence: EvidenceSource::None,
        evaluate: ntp,
    },
    Pattern {
        id: "hardening.auto_updates",
        area: Area::Hardening,
        subject: "Автообновления безопасности",
        description: "Автоматические обновления устанавливают исправления уязвимостей без участия оператора.",
        weight: Weight::Low,
        advice: ADVICE_AUTO_UPDATES,
        evidence: EvidenceSource::Tab(TAB_SUMMARY),
        evaluate: auto_updates,
    },
    Pattern {
        id: "hardening.auditd",
        area: Area::Hardening,
        subject: "Аудит системных вызовов (auditd)",
        description: "auditd фиксирует действия с правами root; без него разбор инцидента опирается на историю shell.",
        weight: Weight::Low,
        advice: ADVICE_AUDITD,
        evidence: EvidenceSource::None,
        evaluate: auditd,
    },
];

fn mac(ctx: &AuditContext<'_>) -> Vec<Verdict> {
    with_security(ctx, |s: &SecuritySnapshot| match &s.hardening.mac {
        Some(MacStatus::SelinuxEnforcing) => Verdict::pass("SELinux enforcing"),
        Some(MacStatus::AppArmor) => Verdict::pass("AppArmor активен"),
        Some(MacStatus::SelinuxPermissive) => Verdict::warn(
            "SELinux в режиме permissive - только пишет в журнал, ничего не блокирует",
        ),
        Some(MacStatus::Disabled) => Verdict::fail("SELinux отключён, AppArmor не найден"),
        Some(MacStatus::Unknown) | None => Verdict::skipped("не обнаружен"),
    })
}

fn ntp(ctx: &AuditContext<'_>) -> Vec<Verdict> {
    with_security(ctx, |s| match s.hardening.ntp_synced {
        Some(true) => Verdict::pass("синхронизировано"),
        Some(false) => Verdict::warn("не синхронизировано (NTPSynchronized=no)"),
        None => Verdict::skipped("timedatectl недоступен"),
    })
}

fn auto_updates(ctx: &AuditContext<'_>) -> Vec<Verdict> {
    with_security(ctx, |s| {
        if s.hardening.auto_updates {
            return Verdict::pass("unattended-upgrades или dnf-automatic активен");
        }
        Verdict::warn("не настроены")
    })
}

fn auditd(ctx: &AuditContext<'_>) -> Vec<Verdict> {
    with_security(ctx, |s| {
        if s.hardening.auditd {
            return Verdict::pass("запущен");
        }
        Verdict::warn("не запущен")
    })
}
