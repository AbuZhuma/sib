use sib_modules::docker::{self, DockerSnapshot};
use sib_modules::security::SecuritySnapshot;

use super::with_security;
use crate::audit::context::AuditContext;
use crate::audit::evidence::EvidenceSource;
use crate::audit::pattern::{Area, Pattern, Verdict, Weight};

const SYSCTL_CONF: &str = "/etc/sysctl.conf";
const ADVICE_SYSCTL: &str =
    "Set the value in /etc/sysctl.d/99-hardening.conf and apply it with sysctl --system.";
const ADVICE_FORWARD: &str = "If the server is not a router and not a container host, set net.ipv4.ip_forward=0 in /etc/sysctl.d/.";

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
        "net.ipv4.tcp_syncookies=1 keeps accepting connections during a SYN flood without filling the queue.",
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
        "Reverse path filter",
        "net.ipv4.conf.all.rp_filter=1 drops packets with a forged source address that arrive on the wrong interface.",
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
        "Accepting ICMP redirects",
        "An ICMP redirect lets a host on the same network change a route. A server does not need such packets, for IPv4 or IPv6.",
        Weight::Low,
        accept_redirects,
    ),
    sysctl_pattern(
        "kernel.send_redirects",
        "Sending ICMP redirects",
        "A server that is not a router should not send redirects.",
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
        "Source routed packets get around the routing rules. accept_source_route must be 0.",
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
        "Logging of suspicious packets",
        "log_martians=1 writes packets with impossible source addresses to the journal.",
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
        "Answering broadcast ping",
        "icmp_echo_ignore_broadcasts=1 stops the server from being used to amplify a smurf attack.",
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
        subject: "Packet forwarding",
        description: "net.ipv4.ip_forward=1 turns on packet forwarding. Docker sets it itself. Without containers or a VPN it is not needed.",
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
        return Verdict::skipped(format!("{} was not read", rule.key));
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
    Verdict::warn(format!("{} = {value}, should be {wanted}", rule.key))
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
        Some("1") if has_docker => Verdict::pass("on, Docker needs it"),
        Some("1") => Verdict::warn("on (net.ipv4.ip_forward = 1), no Docker found"),
        Some(value) => Verdict::pass(format!("net.ipv4.ip_forward = {value}")),
        None => Verdict::skipped("net.ipv4.ip_forward was not read"),
    })
}
