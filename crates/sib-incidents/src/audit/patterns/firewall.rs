use sib_modules::ports::{self, PortsSnapshot, Protocol};
use sib_modules::security::{FirewallState, SecuritySnapshot, Switch};

use super::{security_snapshot, with_security};
use crate::audit::context::{AuditContext, list_or};
use crate::audit::evidence::{EvidenceSource, SECURITY_ATTACKS, TAB_PORTS};
use crate::audit::pattern::{Area, Pattern, Verdict, Weight};

const PUBLIC_WARN: usize = 6;
const PUBLIC_LISTED: usize = 12;
const SHOWN: usize = 6;
const DEFAULT_SSH_PORT: u16 = 22;

const ADVICE_FIREWALL: &str =
    "Turn on nftables, ufw or firewalld and open only the ports you need (SSH, 80, 443).";
const ADVICE_RISKY: &str = "Close the port or bind the service to 127.0.0.1 or an internal network. Such services are published outside through a VPN or TLS with authentication.";
const ADVICE_EXPOSED: &str = "The port listens on all addresses and answers from outside with no firewall rule for it. Close it or limit the source addresses.";
const ADVICE_PUBLIC: &str =
    "Bind service ports to 127.0.0.1. Every public port widens the attack surface.";
const ADVICE_FAIL2BAN: &str =
    "Install fail2ban and turn on the sshd jail. It blocks an address after a few failed attempts.";
const ADVICE_BRUTE_FORCE: &str =
    "Ban the addresses on the Attacks and bans subpage, turn off password login, turn on fail2ban.";
const ADVICE_UNBANNED: &str = "Ban the addresses by hand or turn on fail2ban.";
const ADVICE_SSH_EXPOSED: &str = "Turn off password login and install fail2ban.";

pub static PATTERNS: &[Pattern] = &[
    Pattern {
        id: "firewall.active",
        area: Area::Firewall,
        subject: "Firewall",
        description: "Without a firewall every socket listening on 0.0.0.0 is reachable from outside, service ports included.",
        weight: Weight::High,
        advice: ADVICE_FIREWALL,
        evidence: EvidenceSource::Tab(TAB_PORTS),
        evaluate: firewall,
    },
    Pattern {
        id: "firewall.risky_ports",
        area: Area::Firewall,
        subject: "Risky open ports",
        description: "telnet (23), ftp (21), rsh (512-514), the Docker API (2375, 2376), Redis (6379), MongoDB (27017) and Elasticsearch (9200) run without encryption or without authentication by default and should not be reachable from outside.",
        weight: Weight::High,
        advice: ADVICE_RISKY,
        evidence: EvidenceSource::Tab(TAB_PORTS),
        evaluate: risky_ports,
    },
    Pattern {
        id: "firewall.exposed_ports",
        area: Area::Firewall,
        subject: "Ports open past the firewall",
        description: "The port answers a connection from outside and has no firewall rule.",
        weight: Weight::Medium,
        advice: ADVICE_EXPOSED,
        evidence: EvidenceSource::Tab(TAB_PORTS),
        evaluate: exposed_ports,
    },
    Pattern {
        id: "firewall.public_ports",
        area: Area::Firewall,
        subject: "Number of public ports",
        description: "Every port that listens somewhere other than localhost is another way in that has to be guarded and updated.",
        weight: Weight::Low,
        advice: ADVICE_PUBLIC,
        evidence: EvidenceSource::Tab(TAB_PORTS),
        evaluate: public_ports,
    },
    Pattern {
        id: "firewall.fail2ban",
        area: Area::Firewall,
        subject: "Brute force protection (fail2ban)",
        description: "fail2ban reads the sshd journal and blocks an address after a few failed attempts.",
        weight: Weight::Medium,
        advice: ADVICE_FAIL2BAN,
        evidence: EvidenceSource::Tab(SECURITY_ATTACKS),
        evaluate: fail2ban,
    },
    Pattern {
        id: "firewall.ssh_exposed",
        area: Area::Firewall,
        subject: "SSH with passwords and no protection",
        description: "The SSH port is reachable from outside, password login is allowed and fail2ban is missing. Nothing limits how fast passwords can be guessed.",
        weight: Weight::High,
        advice: ADVICE_SSH_EXPOSED,
        evidence: EvidenceSource::File("/etc/ssh/sshd_config"),
        evaluate: ssh_exposed,
    },
    Pattern {
        id: "firewall.brute_force",
        area: Area::Firewall,
        subject: "Password guessing right now",
        description: "Addresses with ten or more failed attempts in the last 10 minutes.",
        weight: Weight::Medium,
        advice: ADVICE_BRUTE_FORCE,
        evidence: EvidenceSource::Tab(SECURITY_ATTACKS),
        evaluate: brute_force,
    },
    Pattern {
        id: "firewall.unbanned_attackers",
        area: Area::Firewall,
        subject: "Attackers that are not banned",
        description: "Addresses that guess passwords and are not blocked by fail2ban or the firewall.",
        weight: Weight::Medium,
        advice: ADVICE_UNBANNED,
        evidence: EvidenceSource::Tab(SECURITY_ATTACKS),
        evaluate: unbanned_attackers,
    },
];

fn firewall(ctx: &AuditContext<'_>) -> Vec<Verdict> {
    if let Ok(snapshot) = security_snapshot(ctx) {
        match &snapshot.firewall {
            FirewallState::Active(name) => {
                return Verdict::pass(format!("active ({name})")).single();
            }
            FirewallState::Inactive => {
                return Verdict::fail(
                    "not active: ufw, firewalld, nftables and iptables are not running",
                )
                .single();
            }
            FirewallState::Unknown => {}
        }
    }
    let Some(ports) = ctx.data::<PortsSnapshot>(ports::ID) else {
        return Verdict::skipped("no firewall data").single();
    };
    match &ports.firewall {
        Some(firewall) if firewall.active => Verdict::pass(format!(
            "active ({:?}), port rules: {}",
            firewall.backend,
            firewall.allowed.len()
        )),
        Some(firewall) => Verdict::fail(format!(
            "{:?} is installed but not active",
            firewall.backend
        )),
        None => Verdict::skipped("no firewall data"),
    }
    .single()
}

fn risky_ports(ctx: &AuditContext<'_>) -> Vec<Verdict> {
    with_security(ctx, |s: &SecuritySnapshot| {
        let ports = &s.hardening.risky_ports;
        if ports.is_empty() {
            return Verdict::pass(
                "telnet, ftp, rsh, docker api, redis, mongo and elastic are closed",
            );
        }
        let list: Vec<String> = ports.iter().map(u16::to_string).collect();
        Verdict::fail(format!("listening to the outside: {}", list.join(", ")))
    })
}

fn exposed_ports(ctx: &AuditContext<'_>) -> Vec<Verdict> {
    let Some(snapshot) = ctx.data::<PortsSnapshot>(ports::ID) else {
        return Verdict::skipped("the ports module has not collected data").single();
    };
    let exposed: Vec<String> = snapshot
        .exposed_without_firewall()
        .map(|p| format!("{}/{} {}", p.port, p.protocol.label(), p.process_label()))
        .collect();
    if exposed.is_empty() {
        return Verdict::pass("every port reachable from outside is allowed by an explicit rule")
            .single();
    }
    Verdict::warn(format!(
        "reachable from outside with no rule: {}",
        list_or(&exposed[..exposed.len().min(SHOWN)], "")
    ))
    .single()
}

fn public_ports(ctx: &AuditContext<'_>) -> Vec<Verdict> {
    let Some(snapshot) = ctx.data::<PortsSnapshot>(ports::ID) else {
        return Verdict::skipped("the ports module has not collected data").single();
    };
    let mut public: Vec<String> = snapshot
        .public_ports()
        .map(|p| format!("{}/{}", p.port, p.protocol.label()))
        .collect();
    public.sort();
    public.dedup();
    let total = public.len();
    let hidden = total.saturating_sub(PUBLIC_LISTED);
    public.truncate(PUBLIC_LISTED);
    if hidden > 0 {
        public.push(format!("and {hidden} more"));
    }
    Verdict::graded(
        false,
        total >= PUBLIC_WARN,
        format!("{total}: {}", list_or(&public, "-")),
    )
    .single()
}

fn fail2ban(ctx: &AuditContext<'_>) -> Vec<Verdict> {
    with_security(ctx, |s| {
        if s.has_fail2ban {
            let jails: Vec<&str> = s.jails.iter().map(|j| j.name.as_str()).collect();
            return Verdict::pass(format!("active, jails: {}", list_or(&jails, "none")));
        }
        Verdict::warn("not installed or not running")
    })
}

fn ssh_exposed(ctx: &AuditContext<'_>) -> Vec<Verdict> {
    let Ok(security) = security_snapshot(ctx) else {
        return Verdict::skipped(super::NO_SECURITY_DATA).single();
    };
    let Some(ports) = ctx.data::<PortsSnapshot>(ports::ID) else {
        return Verdict::skipped("the ports module has not collected data").single();
    };
    let ssh_port = security.sshd.port.unwrap_or(DEFAULT_SSH_PORT);
    let is_public = ports
        .public_ports()
        .any(|p| p.port == ssh_port && p.protocol == Protocol::Tcp);
    let passwords = security.sshd.password_auth == Some(Switch::On);
    match (is_public, passwords, security.has_fail2ban) {
        (true, true, false) => Verdict::fail(format!(
            "port {ssh_port} is open to the outside, passwords are allowed, no fail2ban"
        )),
        (true, true, true) => Verdict::warn(format!(
            "port {ssh_port} is open to the outside with passwords, only fail2ban holds guessing back"
        )),
        (true, false, _) => {
            Verdict::pass(format!("port {ssh_port} is open to the outside but keys only"))
        }
        (false, _, _) => Verdict::pass(format!("port {ssh_port} is not visible from outside")),
    }
    .single()
}

fn brute_force(ctx: &AuditContext<'_>) -> Vec<Verdict> {
    let Ok(snapshot) = security_snapshot(ctx) else {
        return Verdict::skipped(super::NO_SECURITY_DATA).single();
    };
    let attackers: Vec<String> = snapshot
        .attackers
        .iter()
        .filter(|a| a.is_brute_force())
        .map(|a| {
            format!(
                "{} ({}, {} attempts in 10 min)",
                a.ip,
                ctx.state.country_of(&a.ip).unwrap_or("?"),
                a.recent_failures
            )
        })
        .collect();
    if attackers.is_empty() {
        return Verdict::pass(format!(
            "no active guessing, {} addresses with failed attempts in the last day",
            snapshot.attackers.len()
        ))
        .single();
    }
    Verdict::fail(format!(
        "guessing in progress from {}: {}",
        attackers.len(),
        list_or(&attackers[..attackers.len().min(SHOWN)], "")
    ))
    .single()
}

fn unbanned_attackers(ctx: &AuditContext<'_>) -> Vec<Verdict> {
    with_security(ctx, |s| {
        let unbanned: Vec<&str> = s
            .attackers
            .iter()
            .filter(|a| a.is_brute_force() && !s.is_banned(&a.ip))
            .map(|a| a.ip.as_str())
            .collect();
        if unbanned.is_empty() {
            return Verdict::pass(format!(
                "every attacker is blocked, {} bans in total",
                s.bans.len()
            ));
        }
        Verdict::warn(format!(
            "{} not banned: {}",
            unbanned.len(),
            list_or(&unbanned[..unbanned.len().min(SHOWN)], "")
        ))
    })
}
