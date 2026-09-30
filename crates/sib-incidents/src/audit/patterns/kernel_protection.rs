use super::kernel::{SysctlRule, Wanted, rule, sysctl_pattern, verdict};
use super::with_security;
use crate::audit::context::AuditContext;
use crate::audit::pattern::{Pattern, Verdict, Weight};

pub static PATTERNS: &[Pattern] = &[
    sysctl_pattern(
        "kernel.aslr",
        "ASLR",
        "kernel.randomize_va_space=2 randomizes the addresses of the stack, the heap and libraries. Without it the addresses are predictable.",
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
        "Hiding kernel addresses",
        "kernel.kptr_restrict>=1 hides kernel addresses in /proc and dmesg. They are used to get around KASLR.",
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
        "Access to dmesg",
        "kernel.dmesg_restrict=1 closes the kernel log to ordinary users.",
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
        "ptrace limit",
        "kernel.yama.ptrace_scope>=1 stops a process from attaching a debugger to another process of the same user and reading its memory.",
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
        "Magic SysRq",
        "kernel.sysrq=0 turns off the key combinations that reboot the machine or kill processes from the console.",
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
        "Unprivileged BPF",
        "kernel.unprivileged_bpf_disabled=1 stops ordinary users from loading BPF programs. They are used to raise privileges.",
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
        "Access to perf",
        "kernel.perf_event_paranoid>=2 stops ordinary users from profiling the kernel and other users processes.",
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
        "Symlink protection",
        "fs.protected_symlinks and fs.protected_hardlinks close attacks through symbolic and hard links in world-writable directories.",
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
