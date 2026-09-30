use sib_modules::security::{MacStatus, SecuritySnapshot};

use super::with_security;
use crate::audit::context::AuditContext;
use crate::audit::evidence::{EvidenceSource, TAB_SUMMARY};
use crate::audit::pattern::{Area, Pattern, Verdict, Weight};

const ADVICE_MAC: &str = "Turn on SELinux in enforcing mode, or AppArmor.";
const ADVICE_NTP: &str =
    "Turn on time sync: timedatectl set-ntp true (chrony or systemd-timesyncd).";
const ADVICE_AUTO_UPDATES: &str = "Turn on automatic security updates: unattended-upgrades (Debian, Ubuntu) or dnf-automatic (RHEL, Fedora).";
const ADVICE_AUDITD: &str = "Install auditd and turn it on.";

pub static PATTERNS: &[Pattern] = &[
    Pattern {
        id: "hardening.mac",
        area: Area::Hardening,
        subject: "SELinux / AppArmor",
        description: "Mandatory access control keeps a service inside its profile. Anything outside the profile is denied, even if the service is taken over.",
        weight: Weight::Medium,
        advice: ADVICE_MAC,
        evidence: EvidenceSource::None,
        evaluate: mac,
    },
    Pattern {
        id: "hardening.ntp",
        area: Area::Hardening,
        subject: "Time sync",
        description: "Without time sync, journal timestamps drift apart and TLS certificates and one-time codes stop validating.",
        weight: Weight::Low,
        advice: ADVICE_NTP,
        evidence: EvidenceSource::None,
        evaluate: ntp,
    },
    Pattern {
        id: "hardening.auto_updates",
        area: Area::Hardening,
        subject: "Automatic security updates",
        description: "Automatic updates install security fixes without anyone doing it by hand.",
        weight: Weight::Low,
        advice: ADVICE_AUTO_UPDATES,
        evidence: EvidenceSource::Tab(TAB_SUMMARY),
        evaluate: auto_updates,
    },
    Pattern {
        id: "hardening.auditd",
        area: Area::Hardening,
        subject: "System call audit (auditd)",
        description: "auditd records what is done with root rights. Without it an incident review has only the shell history to go on.",
        weight: Weight::Low,
        advice: ADVICE_AUDITD,
        evidence: EvidenceSource::None,
        evaluate: auditd,
    },
];

fn mac(ctx: &AuditContext<'_>) -> Vec<Verdict> {
    with_security(ctx, |s: &SecuritySnapshot| match &s.hardening.mac {
        Some(MacStatus::SelinuxEnforcing) => Verdict::pass("SELinux enforcing"),
        Some(MacStatus::AppArmor) => Verdict::pass("AppArmor is active"),
        Some(MacStatus::SelinuxPermissive) => {
            Verdict::warn("SELinux is permissive, it only logs and blocks nothing")
        }
        Some(MacStatus::Disabled) => Verdict::fail("SELinux is off and AppArmor was not found"),
        Some(MacStatus::Unknown) | None => Verdict::skipped("not found"),
    })
}

fn ntp(ctx: &AuditContext<'_>) -> Vec<Verdict> {
    with_security(ctx, |s| match s.hardening.ntp_synced {
        Some(true) => Verdict::pass("in sync"),
        Some(false) => Verdict::warn("not in sync (NTPSynchronized=no)"),
        None => Verdict::skipped("timedatectl is not available"),
    })
}

fn auto_updates(ctx: &AuditContext<'_>) -> Vec<Verdict> {
    with_security(ctx, |s| {
        if s.hardening.auto_updates {
            return Verdict::pass("unattended-upgrades or dnf-automatic is active");
        }
        Verdict::warn("not set up")
    })
}

fn auditd(ctx: &AuditContext<'_>) -> Vec<Verdict> {
    with_security(ctx, |s| {
        if s.hardening.auditd {
            return Verdict::pass("running");
        }
        Verdict::warn("not running")
    })
}
