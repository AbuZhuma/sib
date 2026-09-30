use sib_modules::security::SecuritySnapshot;

use super::{NEEDS_SUDO, with_security};
use crate::audit::context::{AuditContext, list_or};
use crate::audit::evidence::{EvidenceSource, SECURITY_ACCESS};
use crate::audit::pattern::{Area, Pattern, Verdict, Weight};

const PASSWORD_METHOD: &str = "password";
const SHOWN: usize = 5;
const SUSPICIOUS_FAILURES: u64 = 5;

const ADVICE_UID0: &str =
    "Change the UID of those accounts or remove them. Only root should have UID 0.";
const ADVICE_EMPTY: &str = "Set a password (passwd <user>) or lock the account (passwd -l <user>).";
const ADVICE_NOPASSWD: &str = "Remove NOPASSWD from /etc/sudoers and /etc/sudoers.d/, keep it only for automation with a narrow set of commands.";
const ADVICE_KEYS: &str = "Run chmod 600 ~/.ssh/authorized_keys and chmod 700 ~/.ssh.";
const ADVICE_WORLD_WRITABLE: &str =
    "Clear the o+w bit. An ordinary user should not be able to change configuration in /etc.";
const ADVICE_ROOT_PASSWORD_LOGINS: &str = "Move root to key login or turn PermitRootLogin off.";
const ADVICE_ATTACKER_LOGIN: &str = "Treat the account as taken over: change the password and the keys, check ~/.ssh, crontab and the running processes.";
const ADVICE_SUDO_FAILURES: &str =
    "Look at the Logins and sudo subpage to see which user ran which commands.";

pub static PATTERNS: &[Pattern] = &[
    Pattern {
        id: "access.extra_uid0",
        area: Area::Access,
        subject: "Accounts with UID 0",
        description: "An account with UID 0 has the same rights as root.",
        weight: Weight::High,
        advice: ADVICE_UID0,
        evidence: EvidenceSource::File("/etc/passwd"),
        evaluate: extra_uid0,
    },
    Pattern {
        id: "access.empty_passwords",
        area: Area::Access,
        subject: "Empty passwords",
        description: "An account with an empty password field in /etc/shadow allows local login without a password, and with PermitEmptyPasswords yes over SSH as well.",
        weight: Weight::High,
        advice: ADVICE_EMPTY,
        evidence: EvidenceSource::File("/etc/shadow"),
        evaluate: empty_passwords,
    },
    Pattern {
        id: "access.sudo_nopasswd",
        area: Area::Access,
        subject: "sudo without a password",
        description: "With a NOPASSWD rule, taking over a user account gives root rights with no password at all.",
        weight: Weight::Low,
        advice: ADVICE_NOPASSWD,
        evidence: EvidenceSource::File("/etc/sudoers"),
        evaluate: sudo_nopasswd,
    },
    Pattern {
        id: "access.writable_keys",
        area: Area::Access,
        subject: "Permissions on authorized_keys",
        description: "If authorized_keys is writable by the group or by everyone, any local process can add a key and get SSH access.",
        weight: Weight::High,
        advice: ADVICE_KEYS,
        evidence: EvidenceSource::None,
        evaluate: writable_keys,
    },
    Pattern {
        id: "access.world_writable_etc",
        area: Area::Access,
        subject: "World-writable files in /etc",
        description: "A configuration file with the o+w bit can be changed by any user or service.",
        weight: Weight::Medium,
        advice: ADVICE_WORLD_WRITABLE,
        evidence: EvidenceSource::None,
        evaluate: world_writable,
    },
    Pattern {
        id: "access.root_password_logins",
        area: Area::Access,
        subject: "Root logins with a password",
        description: "Successful root password logins in the last day. The root account is open to guessing.",
        weight: Weight::Medium,
        advice: ADVICE_ROOT_PASSWORD_LOGINS,
        evidence: EvidenceSource::Tab(SECURITY_ACCESS),
        evaluate: root_password_logins,
    },
    Pattern {
        id: "access.login_from_attacker",
        area: Area::Access,
        subject: "Login from an attacking address",
        description: "An address guessed passwords and then logged in successfully.",
        weight: Weight::High,
        advice: ADVICE_ATTACKER_LOGIN,
        evidence: EvidenceSource::Tab(SECURITY_ACCESS),
        evaluate: login_from_attacker,
    },
    Pattern {
        id: "access.sudo_failures",
        area: Area::Access,
        subject: "sudo denials",
        description: "sudo denials in the last day. A user without rights tried to run a command as root, or got the password wrong several times.",
        weight: Weight::Low,
        advice: ADVICE_SUDO_FAILURES,
        evidence: EvidenceSource::Tab(SECURITY_ACCESS),
        evaluate: sudo_failures,
    },
];

fn extra_uid0(ctx: &AuditContext<'_>) -> Vec<Verdict> {
    with_security(ctx, |s: &SecuritySnapshot| {
        let extra = &s.hardening.extra_uid0;
        if extra.is_empty() {
            return Verdict::pass("only root");
        }
        Verdict::fail(format!(
            "besides root, UID 0 belongs to: {}",
            extra.join(", ")
        ))
    })
}

fn empty_passwords(ctx: &AuditContext<'_>) -> Vec<Verdict> {
    with_security(ctx, |s| match &s.hardening.empty_passwords {
        Some(users) if users.is_empty() => Verdict::pass("none"),
        Some(users) => Verdict::fail(format!("no password: {}", users.join(", "))),
        None => Verdict::skipped(NEEDS_SUDO),
    })
}

fn sudo_nopasswd(ctx: &AuditContext<'_>) -> Vec<Verdict> {
    with_security(ctx, |s| match s.hardening.sudo_nopasswd {
        Some(0) => Verdict::pass("no NOPASSWD rules"),
        Some(count) => Verdict::warn(format!("{count} NOPASSWD rules")),
        None => Verdict::skipped(NEEDS_SUDO),
    })
}

fn writable_keys(ctx: &AuditContext<'_>) -> Vec<Verdict> {
    with_security(ctx, |s| match &s.hardening.writable_keys {
        Some(files) if files.is_empty() => Verdict::pass("writable by the owner only"),
        Some(files) => Verdict::fail(format!("open for writing: {}", files.join(", "))),
        None => Verdict::skipped(NEEDS_SUDO),
    })
}

fn world_writable(ctx: &AuditContext<'_>) -> Vec<Verdict> {
    with_security(ctx, |s| {
        let files = &s.hardening.world_writable_etc;
        if files.is_empty() {
            return Verdict::pass("none");
        }
        Verdict::fail(format!("world-writable: {}", files.join(", ")))
    })
}

fn root_password_logins(ctx: &AuditContext<'_>) -> Vec<Verdict> {
    with_security(ctx, |s| {
        let from: Vec<&str> = s
            .logins
            .iter()
            .filter(|l| l.is_root() && l.method == PASSWORD_METHOD)
            .map(|l| l.from.as_str())
            .collect();
        if from.is_empty() {
            return Verdict::pass("none in the last day");
        }
        Verdict::warn(format!(
            "{} in the last day, from: {}",
            from.len(),
            list_or(&from[..from.len().min(SHOWN)], "")
        ))
    })
}

fn login_from_attacker(ctx: &AuditContext<'_>) -> Vec<Verdict> {
    with_security(ctx, |s| {
        let mut suspicious: Vec<String> = Vec::new();
        for login in &s.logins {
            let Some(attacker) = s
                .attackers
                .iter()
                .find(|a| a.ip == login.from && a.failures >= SUSPICIOUS_FAILURES)
            else {
                continue;
            };
            let entry = format!(
                "{} from {} ({}, {} failed attempts from this address)",
                login.user, login.from, login.method, attacker.failures
            );
            if !suspicious.contains(&entry) {
                suspicious.push(entry);
            }
        }
        if suspicious.is_empty() {
            return Verdict::pass("no matches with attacking addresses");
        }
        Verdict::fail(format!(
            "successful login from an address that was guessing passwords: {}",
            list_or(&suspicious[..suspicious.len().min(SHOWN)], "")
        ))
    })
}

fn sudo_failures(ctx: &AuditContext<'_>) -> Vec<Verdict> {
    with_security(ctx, |s| {
        let failed: Vec<String> = s
            .sudo_calls
            .iter()
            .filter(|c| !c.is_success)
            .map(|c| format!("{}: {}", c.user, c.command))
            .collect();
        if failed.is_empty() {
            return Verdict::pass("no denials in the last day");
        }
        Verdict::warn(format!(
            "{} denials in the last day: {}",
            failed.len(),
            list_or(&failed[..failed.len().min(SHOWN)], "")
        ))
    })
}
