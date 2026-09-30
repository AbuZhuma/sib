use super::kernel::{SysctlRule, Wanted, rule, sysctl_pattern, verdict};
use super::with_security;
use crate::audit::context::AuditContext;
use crate::audit::pattern::{Pattern, Verdict, Weight};

pub static PATTERNS: &[Pattern] = &[
    sysctl_pattern(
        "kernel.aslr",
        "ASLR",
        "kernel.randomize_va_space=2 рандомизирует адреса стека, кучи и библиотек; без этого адреса предсказуемы.",
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
        "kernel.kptr_restrict>=1 скрывает адреса ядра в /proc и dmesg; по ним обходят KASLR.",
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
        "kernel.dmesg_restrict=1 закрывает журнал ядра от обычных пользователей.",
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
        "kernel.yama.ptrace_scope>=1 запрещает процессу подключаться отладчиком к другому процессу того же пользователя и читать его память.",
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
        "kernel.unprivileged_bpf_disabled=1 запрещает обычным пользователям загружать BPF-программы; через них повышают привилегии.",
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
        "fs.protected_symlinks и fs.protected_hardlinks закрывают атаки через символические и жёсткие ссылки в общедоступных каталогах.",
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
