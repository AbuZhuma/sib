use asiba_modules::docker::{self, DockerSnapshot};
use asiba_modules::security::SecuritySnapshot;

use super::with_security;
use crate::audit::context::AuditContext;
use crate::audit::evidence::EvidenceSource;
use crate::audit::pattern::{Area, Pattern, Verdict, Weight};

const SYSCTL_CONF: &str = "/etc/sysctl.conf";
const ADVICE_SYSCTL: &str =
    "Задайте параметр в /etc/sysctl.d/99-hardening.conf и примените sysctl --system.";
const ADVICE_FORWARD: &str = "Если сервер не маршрутизатор и не хост контейнеров, задайте net.ipv4.ip_forward=0 в /etc/sysctl.d/.";

pub(super) struct SysctlRule {
    pub(super) key: &'static str,
    pub(super) wanted: Wanted,
}

pub(super) enum Wanted {
    Exactly(&'static str),
    AtLeast(u32),
}

pub(super) const fn sysctl_pattern(
    id: &'static str,
    subject: &'static str,
    description: &'static str,
    weight: Weight,
    evaluate: fn(&AuditContext<'_>) -> Vec<Verdict>,
) -> Pattern {
    Pattern {
        id,
        area: Area::Kernel,
        subject,
        description,
        weight,
        advice: ADVICE_SYSCTL,
        evidence: EvidenceSource::File(SYSCTL_CONF),
        evaluate,
    }
}

pub static PATTERNS: &[Pattern] = &[
    sysctl_pattern(
        "kernel.syncookies",
        "SYN cookies",
        "net.ipv4.tcp_syncookies=1 позволяет принимать соединения во время SYN-флуда, не исчерпывая очередь.",
        Weight::Medium,
        |ctx| {
            rule(
                ctx,
                SysctlRule {
                    key: "net.ipv4.tcp_syncookies",
                    wanted: Wanted::Exactly("1"),
                },
            )
        },
    ),
    sysctl_pattern(
        "kernel.rp_filter",
        "Фильтр обратного пути",
        "net.ipv4.conf.all.rp_filter=1 отбрасывает пакеты с подделанным адресом источника, пришедшие не с того интерфейса.",
        Weight::Low,
        |ctx| {
            rule(
                ctx,
                SysctlRule {
                    key: "net.ipv4.conf.all.rp_filter",
                    wanted: Wanted::Exactly("1"),
                },
            )
        },
    ),
    sysctl_pattern(
        "kernel.accept_redirects",
        "Приём ICMP-редиректов",
        "ICMP redirect позволяет узлу той же сети подменить маршрут; серверу такие пакеты не нужны (IPv4 и IPv6).",
        Weight::Low,
        accept_redirects,
    ),
    sysctl_pattern(
        "kernel.send_redirects",
        "Отправка ICMP-редиректов",
        "Сервер, не являющийся маршрутизатором, не должен рассылать редиректы.",
        Weight::Low,
        |ctx| {
            rule(
                ctx,
                SysctlRule {
                    key: "net.ipv4.conf.all.send_redirects",
                    wanted: Wanted::Exactly("0"),
                },
            )
        },
    ),
    sysctl_pattern(
        "kernel.source_route",
        "Source routing",
        "Пакеты с маршрутом от источника обходят правила маршрутизации; accept_source_route должен быть 0.",
        Weight::Low,
        |ctx| {
            rule(
                ctx,
                SysctlRule {
                    key: "net.ipv4.conf.all.accept_source_route",
                    wanted: Wanted::Exactly("0"),
                },
            )
        },
    ),
    sysctl_pattern(
        "kernel.log_martians",
        "Журнал подозрительных пакетов",
        "log_martians=1 записывает в журнал пакеты с некорректными адресами источника.",
        Weight::Low,
        |ctx| {
            rule(
                ctx,
                SysctlRule {
                    key: "net.ipv4.conf.all.log_martians",
                    wanted: Wanted::Exactly("1"),
                },
            )
        },
    ),
    sysctl_pattern(
        "kernel.broadcast_ping",
        "Ответ на broadcast-ping",
        "icmp_echo_ignore_broadcasts=1 не даёт использовать сервер как усилитель smurf-атаки.",
        Weight::Low,
        |ctx| {
            rule(
                ctx,
                SysctlRule {
                    key: "net.ipv4.icmp_echo_ignore_broadcasts",
                    wanted: Wanted::Exactly("1"),
                },
            )
        },
    ),
    Pattern {
        id: "kernel.ip_forward",
        area: Area::Kernel,
        subject: "Маршрутизация пакетов",
        description: "net.ipv4.ip_forward=1 включает маршрутизацию пакетов. Docker задаёт его сам; без контейнеров и VPN параметр не нужен.",
        weight: Weight::Low,
        advice: ADVICE_FORWARD,
        evidence: EvidenceSource::File(SYSCTL_CONF),
        evaluate: ip_forward,
    },
];

pub(super) fn rule(ctx: &AuditContext<'_>, rule: SysctlRule) -> Vec<Verdict> {
    with_security(ctx, |s: &SecuritySnapshot| verdict(s, &rule))
}

pub(super) fn verdict(snapshot: &SecuritySnapshot, rule: &SysctlRule) -> Verdict {
    let Some(value) = snapshot.hardening.sysctl(rule.key) else {
        return Verdict::skipped(format!("{} не прочитан", rule.key));
    };
    let is_ok = match rule.wanted {
        Wanted::Exactly(wanted) => value == wanted,
        Wanted::AtLeast(min) => value.parse::<u32>().is_ok_and(|v| v >= min),
    };
    if is_ok {
        return Verdict::pass(format!("{} = {value}", rule.key));
    }
    let wanted = match rule.wanted {
        Wanted::Exactly(wanted) => wanted.to_owned(),
        Wanted::AtLeast(min) => format!(">= {min}"),
    };
    Verdict::warn(format!("{} = {value}, нужно {wanted}", rule.key))
}

fn accept_redirects(ctx: &AuditContext<'_>) -> Vec<Verdict> {
    with_security(ctx, |s| {
        let v4 = verdict(
            s,
            &SysctlRule {
                key: "net.ipv4.conf.all.accept_redirects",
                wanted: Wanted::Exactly("0"),
            },
        );
        let v6 = verdict(
            s,
            &SysctlRule {
                key: "net.ipv6.conf.all.accept_redirects",
                wanted: Wanted::Exactly("0"),
            },
        );
        if v6.outcome > v4.outcome { v6 } else { v4 }
    })
}

fn ip_forward(ctx: &AuditContext<'_>) -> Vec<Verdict> {
    let has_docker = ctx.data::<DockerSnapshot>(docker::ID).is_some();
    with_security(ctx, |s| match s.hardening.sysctl("net.ipv4.ip_forward") {
        Some("1") if has_docker => Verdict::pass("включена, требуется Docker"),
        Some("1") => Verdict::warn("включена (net.ipv4.ip_forward = 1), Docker не обнаружен"),
        Some(value) => Verdict::pass(format!("net.ipv4.ip_forward = {value}")),
        None => Verdict::skipped("net.ipv4.ip_forward не прочитан"),
    })
}
