use asiba_modules::security::{MacStatus, SecuritySnapshot};

use super::with_security;
use crate::audit::context::AuditContext;
use crate::audit::evidence::{EvidenceSource, TAB_SUMMARY};
use crate::audit::pattern::{Area, Pattern, Verdict, Weight};

const ADVICE_MAC: &str = "Включите SELinux (enforcing) или AppArmor - они ограничивают, что может сделать взломанный сервис.";
const ADVICE_NTP: &str =
    "Включите синхронизацию времени: timedatectl set-ntp true (chrony или systemd-timesyncd).";
const ADVICE_AUTO_UPDATES: &str = "Включите автоматические обновления безопасности: unattended-upgrades (Debian/Ubuntu) или dnf-automatic (RHEL/Fedora).";
const ADVICE_AUDITD: &str = "Установите и включите auditd - он ведёт журнал вызовов sudo, изменений файлов и входов, который нельзя подделать из userspace.";

pub static PATTERNS: &[Pattern] = &[
    Pattern {
        id: "hardening.mac",
        area: Area::Hardening,
        subject: "SELinux / AppArmor",
        description: "Мандатный контроль доступа ограничивает сервисы их профилем: взломанный nginx не сможет читать /etc/shadow или запускать шелл.",
        weight: Weight::Medium,
        advice: ADVICE_MAC,
        evidence: EvidenceSource::None,
        evaluate: mac,
    },
    Pattern {
        id: "hardening.ntp",
        area: Area::Hardening,
        subject: "Синхронизация времени",
        description: "Без точного времени не сходятся журналы, ломаются TLS-сертификаты и двухфакторные коды.",
        weight: Weight::Low,
        advice: ADVICE_NTP,
        evidence: EvidenceSource::None,
        evaluate: ntp,
    },
    Pattern {
        id: "hardening.auto_updates",
        area: Area::Hardening,
        subject: "Автообновления безопасности",
        description: "Уязвимости в openssl, sudo и ядре закрываются пакетами; автообновления ставят их без участия человека.",
        weight: Weight::Low,
        advice: ADVICE_AUTO_UPDATES,
        evidence: EvidenceSource::Tab(TAB_SUMMARY),
        evaluate: auto_updates,
    },
    Pattern {
        id: "hardening.auditd",
        area: Area::Hardening,
        subject: "Аудит системных вызовов (auditd)",
        description: "auditd фиксирует, кто и что делал с правами root, - без него разбор инцидента опирается только на историю shell.",
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
        Verdict::warn("не настроены - обновления безопасности ставятся только вручную")
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
