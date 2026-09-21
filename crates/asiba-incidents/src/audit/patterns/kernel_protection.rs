use super::kernel::{SysctlRule, Wanted, rule, sysctl_pattern, verdict};
use super::with_security;
use crate::audit::context::AuditContext;
use crate::audit::pattern::{Pattern, Verdict, Weight};

pub static PATTERNS: &[Pattern] = &[
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
];

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
