use sib_modules::security::{SecuritySnapshot, Switch};

use super::{NEEDS_SUDO, with_security};
use crate::audit::context::AuditContext;
use crate::audit::evidence::EvidenceSource;
use crate::audit::pattern::{Area, Outcome, Pattern, Verdict, Weight};

const SSHD_CONFIG: &str = "/etc/ssh/sshd_config";
const MAX_AUTH_TRIES: u32 = 4;
const MAX_LOGIN_GRACE_SECS: u32 = 60;
const DEFAULT_PORT: u16 = 22;

const ADVICE_SSHD: &str = "Change the option in /etc/ssh/sshd_config (or in sshd_config.d/*.conf) and restart sshd: systemctl restart sshd.";

const fn sshd_pattern(
    id: &'static str,
    subject: &'static str,
    description: &'static str,
    weight: Weight,
    evaluate: fn(&AuditContext<'_>) -> Vec<Verdict>,
) -> Pattern {
    Pattern {
        id,
        area: Area::Ssh,
        subject,
        description,
        weight,
        advice: ADVICE_SSHD,
        evidence: EvidenceSource::File(SSHD_CONFIG),
        evaluate,
    }
}

pub static PATTERNS: &[Pattern] = &[
    sshd_pattern(
        "ssh.password_auth",
        "Password login",
        "Passwords get guessed by brute force. PasswordAuthentication no leaves only key login.",
        Weight::High,
        password_auth,
    ),
    sshd_pattern(
        "ssh.root_login",
        "Root login over SSH",
        "The root account exists on every server and is the first one attacked. PermitRootLogin no makes people log in as a normal user and raise rights with sudo.",
        Weight::High,
        root_login,
    ),
    sshd_pattern(
        "ssh.pubkey_auth",
        "Key login",
        "PubkeyAuthentication yes is needed so that passwords can be turned off without losing access.",
        Weight::Medium,
        pubkey_auth,
    ),
    sshd_pattern(
        "ssh.empty_passwords",
        "Empty passwords",
        "PermitEmptyPasswords yes allows login with no password at all.",
        Weight::High,
        empty_passwords,
    ),
    sshd_pattern(
        "ssh.max_auth_tries",
        "Attempt limit per connection",
        "MaxAuthTries limits how many password attempts fit in one connection. A high value speeds up guessing.",
        Weight::Low,
        max_auth_tries,
    ),
    sshd_pattern(
        "ssh.login_grace_time",
        "Time to enter the password",
        "LoginGraceTime sets how many seconds an unauthenticated connection is kept. A high value lets someone take all sshd slots.",
        Weight::Low,
        login_grace_time,
    ),
    sshd_pattern(
        "ssh.max_startups",
        "Limit of parallel connections",
        "MaxStartups sets how many unauthenticated connections sshd keeps at once. It is the built-in brake on guessing.",
        Weight::Low,
        max_startups,
    ),
    sshd_pattern(
        "ssh.x11_forwarding",
        "X11 forwarding",
        "On a server without a desktop X11Forwarding is not needed and only widens the attack surface.",
        Weight::Low,
        x11_forwarding,
    ),
    sshd_pattern(
        "ssh.tcp_forwarding",
        "TCP port forwarding",
        "AllowTcpForwarding yes lets any logged-in user tunnel traffic through the server, including to internal services.",
        Weight::Low,
        tcp_forwarding,
    ),
    sshd_pattern(
        "ssh.client_alive",
        "Dropping stale sessions",
        "ClientAliveInterval makes sshd close sessions whose client stopped answering. Without it they stay open.",
        Weight::Low,
        client_alive,
    ),
    sshd_pattern(
        "ssh.port",
        "SSH port",
        "Port 22 is scanned first. Changing the port is not protection by itself, but it cuts the number of attempts and journal noise.",
        Weight::Low,
        port,
    ),
];

fn switch(
    value: Option<Switch>,
    option: &str,
    wanted: Switch,
    bad: Outcome,
    problem: &str,
) -> Verdict {
    let state = |v: Switch| match v {
        Switch::On => "yes",
        Switch::Off => "no",
        Switch::Unknown => "?",
    };
    match value {
        Some(v) if v == wanted => Verdict::pass(format!("{option} {}", state(v))),
        Some(v) => Verdict::new(bad, format!("{problem} ({option} {})", state(v))),
        None => Verdict::skipped(NEEDS_SUDO),
    }
}

fn password_auth(ctx: &AuditContext<'_>) -> Vec<Verdict> {
    with_security(ctx, |s: &SecuritySnapshot| {
        switch(
            s.sshd.password_auth,
            "PasswordAuthentication",
            Switch::Off,
            Outcome::Fail,
            "passwords are accepted, the server is open to guessing",
        )
    })
}

fn root_login(ctx: &AuditContext<'_>) -> Vec<Verdict> {
    with_security(ctx, |s| match s.sshd.permit_root_login.as_deref() {
        Some("yes") => Verdict::fail("root can log in with a password (PermitRootLogin yes)"),
        Some("no") => Verdict::pass("not allowed (PermitRootLogin no)"),
        Some(value) => Verdict::warn(format!(
            "root can log in with a key (PermitRootLogin {value})"
        )),
        None => Verdict::skipped(NEEDS_SUDO),
    })
}

fn pubkey_auth(ctx: &AuditContext<'_>) -> Vec<Verdict> {
    with_security(ctx, |s| {
        switch(
            s.sshd.pubkey_auth,
            "PubkeyAuthentication",
            Switch::On,
            Outcome::Fail,
            "keys are not accepted, only passwords are left",
        )
    })
}

fn empty_passwords(ctx: &AuditContext<'_>) -> Vec<Verdict> {
    with_security(ctx, |s| {
        switch(
            s.sshd.permit_empty_passwords,
            "PermitEmptyPasswords",
            Switch::Off,
            Outcome::Fail,
            "login without a password is allowed",
        )
    })
}

fn max_auth_tries(ctx: &AuditContext<'_>) -> Vec<Verdict> {
    with_security(ctx, |s| match s.sshd.max_auth_tries {
        Some(tries) if tries <= MAX_AUTH_TRIES => Verdict::pass(format!("MaxAuthTries {tries}")),
        Some(tries) => Verdict::warn(format!(
            "{tries} attempts per connection (MaxAuthTries), {MAX_AUTH_TRIES} or fewer is better"
        )),
        None => Verdict::skipped(NEEDS_SUDO),
    })
}

fn login_grace_time(ctx: &AuditContext<'_>) -> Vec<Verdict> {
    with_security(ctx, |s| match s.sshd.login_grace_time {
        Some(secs) if secs > 0 && secs <= MAX_LOGIN_GRACE_SECS => {
            Verdict::pass(format!("LoginGraceTime {secs}"))
        }
        Some(0) => Verdict::warn("no limit (LoginGraceTime 0)"),
        Some(secs) => Verdict::warn(format!(
            "{secs} s to enter the password (LoginGraceTime), {MAX_LOGIN_GRACE_SECS} is better"
        )),
        None => Verdict::skipped(NEEDS_SUDO),
    })
}

fn max_startups(ctx: &AuditContext<'_>) -> Vec<Verdict> {
    with_security(ctx, |s| match s.sshd.max_startups.as_deref() {
        Some(value) if value.contains(':') => Verdict::pass(format!("MaxStartups {value}")),
        Some(value) => Verdict::warn(format!(
            "no random drop of connections (MaxStartups {value}), 10:30:100 is better"
        )),
        None => Verdict::skipped(NEEDS_SUDO),
    })
}

fn x11_forwarding(ctx: &AuditContext<'_>) -> Vec<Verdict> {
    with_security(ctx, |s| {
        switch(
            s.sshd.x11_forwarding,
            "X11Forwarding",
            Switch::Off,
            Outcome::Warn,
            "X11 forwarding is on",
        )
    })
}

fn tcp_forwarding(ctx: &AuditContext<'_>) -> Vec<Verdict> {
    with_security(ctx, |s| {
        switch(
            s.sshd.allow_tcp_forwarding,
            "AllowTcpForwarding",
            Switch::Off,
            Outcome::Warn,
            "any user can tunnel traffic",
        )
    })
}

fn client_alive(ctx: &AuditContext<'_>) -> Vec<Verdict> {
    with_security(ctx, |s| match s.sshd.client_alive_interval {
        Some(0) => Verdict::warn("ClientAliveInterval 0, idle sessions are never closed"),
        Some(secs) => Verdict::pass(format!("ClientAliveInterval {secs}")),
        None => Verdict::skipped(NEEDS_SUDO),
    })
}

fn port(ctx: &AuditContext<'_>) -> Vec<Verdict> {
    with_security(ctx, |s| match s.sshd.port {
        Some(DEFAULT_PORT) => Verdict::warn("port 22 (the default)"),
        Some(port) => Verdict::pass(format!("port {port}")),
        None => Verdict::skipped(NEEDS_SUDO),
    })
}
