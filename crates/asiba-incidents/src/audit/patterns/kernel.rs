use asiba_modules::docker::{self, DockerSnapshot};
use asiba_modules::security::SecuritySnapshot;

use super::with_security;
use crate::audit::context::AuditContext;
use crate::audit::evidence::EvidenceSource;
use crate::audit::pattern::{Area, Pattern, Verdict, Weight};

const SYSCTL_CONF: &str = "/etc/sysctl.conf";
const ADVICE_SYSCTL: &str =
    "Задайте параметр в /etc/sysctl.d/99-hardening.conf и примените sysctl --system.";
const ADVICE_FORWARD: &str = "Если сервер не маршрутизатор и не хост контейнеров, выключите net.ipv4.ip_forward=0 в /etc/sysctl.d/.";

struct SysctlRule {
    key: &'static str,
    wanted: Wanted,
}

enum Wanted {
    Exactly(&'static str),
    AtLeast(u32),
}

const fn sysctl_pattern(
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
        "kernel.aslr",
        "ASLR",
        "kernel.randomize_va_space=2 рандомизирует адреса стека, кучи и библиотек - без этого эксплойты переполнений работают надёжно.",
        Weight::Medium,
        |ctx| {
            rule(
                ctx,
                SysctlRule {
                    key: "kernel.randomize_va_space",
                    wanted: Wanted::Exactly("2"),
                },
            )
        },
    ),
    sysctl_pattern(
        "kernel.accept_redirects",
        "Приём ICMP-редиректов",
        "ICMP redirect позволяет соседу по сети подменить маршрут - на сервере их принимать не нужно (IPv4 и IPv6).",
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
        "Пакеты с маршрутом от источника позволяют обойти правила маршрутизации - accept_source_route должен быть 0.",
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
        "log_martians=1 пишет в журнал пакеты с невозможными адресами - полезно при разборе атак.",
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
    sysctl_pattern(
        "kernel.kptr_restrict",
        "Скрытие адресов ядра",
        "kernel.kptr_restrict>=1 прячет адреса ядра из /proc и dmesg - они нужны для обхода KASLR.",
        Weight::Low,
        |ctx| {
            rule(
                ctx,
                SysctlRule {
                    key: "kernel.kptr_restrict",
                    wanted: Wanted::AtLeast(1),
                },
            )
        },
    ),
    sysctl_pattern(
        "kernel.dmesg_restrict",
        "Доступ к dmesg",
        "kernel.dmesg_restrict=1 закрывает журнал ядра от обычных пользователей - там адреса, железо и ошибки драйверов.",
        Weight::Low,
        |ctx| {
            rule(
                ctx,
                SysctlRule {
                    key: "kernel.dmesg_restrict",
                    wanted: Wanted::Exactly("1"),
                },
            )
        },
    ),
    sysctl_pattern(
        "kernel.ptrace_scope",
        "Ограничение ptrace",
        "kernel.yama.ptrace_scope>=1 запрещает процессам подключаться отладчиком к чужим процессам того же пользователя и красть из них секреты.",
        Weight::Low,
        |ctx| {
            rule(
                ctx,
                SysctlRule {
                    key: "kernel.yama.ptrace_scope",
                    wanted: Wanted::AtLeast(1),
                },
            )
        },
    ),
    sysctl_pattern(
        "kernel.sysrq",
        "Магический SysRq",
        "kernel.sysrq=0 отключает комбинации, которыми можно перезагрузить или убить процессы с консоли.",
        Weight::Low,
        |ctx| {
            rule(
                ctx,
                SysctlRule {
                    key: "kernel.sysrq",
                    wanted: Wanted::Exactly("0"),
                },
            )
        },
    ),
    sysctl_pattern(
        "kernel.unprivileged_bpf",
        "BPF без привилегий",
        "kernel.unprivileged_bpf_disabled=1 закрывает обычным пользователям загрузку BPF-программ - частый вектор эскалации.",
        Weight::Low,
        |ctx| {
            rule(
                ctx,
                SysctlRule {
                    key: "kernel.unprivileged_bpf_disabled",
                    wanted: Wanted::AtLeast(1),
                },
            )
        },
    ),
    sysctl_pattern(
        "kernel.perf_paranoid",
        "Доступ к perf",
        "kernel.perf_event_paranoid>=2 не даёт обычным пользователям профилировать ядро и чужие процессы.",
        Weight::Low,
        |ctx| {
            rule(
                ctx,
                SysctlRule {
                    key: "kernel.perf_event_paranoid",
                    wanted: Wanted::AtLeast(2),
                },
            )
        },
    ),
    sysctl_pattern(
        "kernel.protected_links",
        "Защита символических ссылок",
        "fs.protected_symlinks и fs.protected_hardlinks закрывают классические атаки через ссылки в /tmp.",
        Weight::Low,
        protected_links,
    ),
    Pattern {
        id: "kernel.ip_forward",
        area: Area::Kernel,
        subject: "Маршрутизация пакетов",
        description: "net.ipv4.ip_forward=1 превращает сервер в маршрутизатор. Docker включает его сам; без контейнеров и VPN это лишнее.",
        weight: Weight::Low,
        advice: ADVICE_FORWARD,
        evidence: EvidenceSource::File(SYSCTL_CONF),
        evaluate: ip_forward,
    },
];

fn rule(ctx: &AuditContext<'_>, rule: SysctlRule) -> Vec<Verdict> {
    with_security(ctx, |s: &SecuritySnapshot| verdict(s, &rule))
}

fn verdict(snapshot: &SecuritySnapshot, rule: &SysctlRule) -> Verdict {
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

fn protected_links(ctx: &AuditContext<'_>) -> Vec<Verdict> {
    with_security(ctx, |s| {
        let symlinks = verdict(
            s,
            &SysctlRule {
                key: "fs.protected_symlinks",
                wanted: Wanted::Exactly("1"),
            },
        );
        let hardlinks = verdict(
            s,
            &SysctlRule {
                key: "fs.protected_hardlinks",
                wanted: Wanted::Exactly("1"),
            },
        );
        if symlinks.outcome >= hardlinks.outcome {
            symlinks
        } else {
            hardlinks
        }
    })
}

fn ip_forward(ctx: &AuditContext<'_>) -> Vec<Verdict> {
    let has_docker = ctx.data::<DockerSnapshot>(docker::ID).is_some();
    with_security(ctx, |s| match s.hardening.sysctl("net.ipv4.ip_forward") {
        Some("1") if has_docker => Verdict::pass("включена, нужна Docker"),
        Some("1") => {
            Verdict::warn("включена (net.ipv4.ip_forward = 1), а контейнеров и VPN не видно")
        }
        Some(value) => Verdict::pass(format!("net.ipv4.ip_forward = {value}")),
        None => Verdict::skipped("net.ipv4.ip_forward не прочитан"),
    })
}
