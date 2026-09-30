use chrono::{DateTime, Utc};
use sib_core::ModuleError;

use super::bans::{iptables_bans, jail_bans, jails, nft_bans, ufw_bans};
use super::hardening;
use super::journal;
use super::model::{BanBackend, FirewallState, SecuritySnapshot, SshdSettings, Switch};
use crate::common::sections::Sections;

const FIREWALL_UNITS: [&str; 5] = [
    "firewalld",
    "ufw",
    "nftables",
    "iptables",
    "netfilter-persistent",
];
const MAX_LOGINS: usize = 50;
const MAX_SUDO_CALLS: usize = 50;

pub fn security_snapshot(raw: &str, now: DateTime<Utc>) -> Result<SecuritySnapshot, ModuleError> {
    let sections = Sections::parse(raw);
    let units = sections.get_or_empty("units");
    if units.trim().is_empty() && sections.get("ssh").is_none() {
        return Err(ModuleError::Parse("empty security output".to_owned()));
    }
    let activity = journal::ssh_activity(sections.get_or_empty("ssh"), now);
    let mut sudo_calls = journal::sudo_calls(sections.get_or_empty("sudo"), now);
    sudo_calls.truncate(MAX_SUDO_CALLS);
    let mut logins = activity.logins;
    logins.truncate(MAX_LOGINS);
    let tools = tools(sections.get_or_empty("tools"));
    let jails = jails(sections.get_or_empty("fail2ban"));
    let mut bans = jail_bans(&jails);
    bans.extend(nft_bans(sections.get_or_empty("nftbans")));
    bans.extend(iptables_bans(sections.get_or_empty("iptbans")));
    bans.extend(ufw_bans(sections.get_or_empty("ufwbans")));
    Ok(SecuritySnapshot {
        attackers: activity.attackers,
        failed_logins: activity.failed_logins,
        logins,
        sudo_calls,
        has_fail2ban: unit_active(units, "fail2ban") || !jails.is_empty(),
        jails,
        sshd: sshd_settings(sections.get_or_empty("sshd")),
        firewall: firewall(units),
        file_hashes: hashes(sections.get_or_empty("hashes")),
        bans,
        ban_backend: ban_backend(&tools),
        is_root_view: sections.get_or_empty("whoami").trim() == "root",
        hardening: hardening::parse(&sections, units),
        slow_collected_at: now,
    })
}

pub fn carry_slow_part(snapshot: &mut SecuritySnapshot, previous: &SecuritySnapshot) {
    snapshot.sshd = previous.sshd.clone();
    snapshot.file_hashes = previous.file_hashes.clone();
    snapshot.ban_backend = previous.ban_backend;
    snapshot.hardening = previous.hardening.clone();
    snapshot.slow_collected_at = previous.slow_collected_at;
}

fn unit_active(units: &str, unit: &str) -> bool {
    units
        .lines()
        .filter_map(|l| l.split_once(' '))
        .any(|(name, state)| name == unit && state.trim() == "active")
}

fn firewall(units: &str) -> FirewallState {
    if units.trim().is_empty() {
        return FirewallState::Unknown;
    }
    FIREWALL_UNITS
        .iter()
        .find(|unit| unit_active(units, unit))
        .map(|unit| FirewallState::Active((*unit).to_owned()))
        .unwrap_or(FirewallState::Inactive)
}

fn tools(raw: &str) -> Vec<String> {
    raw.lines()
        .map(str::trim)
        .filter(|l| !l.is_empty())
        .map(str::to_owned)
        .collect()
}

fn ban_backend(tools: &[String]) -> Option<BanBackend> {
    let has = |name: &str| tools.iter().any(|t| t == name);
    if has("fail2ban-client") {
        return Some(BanBackend::Fail2ban);
    }
    if has("nft") {
        return Some(BanBackend::Nftables);
    }
    if has("iptables") {
        return Some(BanBackend::Iptables);
    }
    has("ufw").then_some(BanBackend::Ufw)
}

fn sshd_settings(raw: &str) -> SshdSettings {
    let mut settings = SshdSettings::default();
    for line in raw.lines() {
        let line = match line.split_once(':') {
            Some((prefix, rest)) if !prefix.contains(' ') => rest,
            _ => line,
        };
        let mut tokens = line.split_whitespace();
        let (Some(key), Some(value)) = (tokens.next(), tokens.next()) else {
            continue;
        };
        match key.to_ascii_lowercase().as_str() {
            "passwordauthentication" => settings.password_auth = Some(Switch::from_yes_no(value)),
            "pubkeyauthentication" => settings.pubkey_auth = Some(Switch::from_yes_no(value)),
            "permitrootlogin" => settings.permit_root_login = Some(value.to_ascii_lowercase()),
            "port" => settings.port = value.parse().ok(),
            "maxauthtries" => settings.max_auth_tries = value.parse().ok(),
            "permitemptypasswords" => {
                settings.permit_empty_passwords = Some(Switch::from_yes_no(value));
            }
            "x11forwarding" => settings.x11_forwarding = Some(Switch::from_yes_no(value)),
            "logingracetime" => settings.login_grace_time = value.parse().ok(),
            "clientaliveinterval" => settings.client_alive_interval = value.parse().ok(),
            "allowtcpforwarding" => {
                settings.allow_tcp_forwarding = Some(Switch::from_yes_no(value));
            }
            "usepam" => settings.use_pam = Some(Switch::from_yes_no(value)),
            "maxstartups" => settings.max_startups = Some(value.to_owned()),
            _ => {}
        }
    }
    settings
}

fn hashes(raw: &str) -> Vec<(String, String)> {
    raw.lines()
        .filter_map(|l| {
            let (hash, path) = l.split_once("  ")?;
            Some((path.trim().to_owned(), hash.trim().to_owned()))
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;

    fn now() -> DateTime<Utc> {
        Utc.with_ymd_and_hms(2026, 9, 14, 12, 0, 0)
            .single()
            .expect("now")
    }

    #[test]
    fn server_fixture_aggregates_attackers_and_logins() {
        let raw = include_str!("../../fixtures/security/server.txt");
        let snapshot = security_snapshot(raw, now()).expect("snapshot");
        let top = &snapshot.attackers[0];
        assert_eq!(top.ip, "192.0.2.10");
        assert_eq!(top.failures, 5);
        assert_eq!(top.users, vec!["root", "admin"]);
        assert_eq!(snapshot.failed_logins, 7);
        assert_eq!(snapshot.logins.len(), 2);
        assert_eq!(snapshot.logins[0].user, "root");
        assert_eq!(snapshot.logins[0].method, "password");
        assert_eq!(snapshot.sudo_calls.len(), 2);
        assert!(!snapshot.sudo_calls[0].is_success);
    }

    #[test]
    fn server_fixture_reads_fail2ban_sshd_and_firewall() {
        let raw = include_str!("../../fixtures/security/server.txt");
        let snapshot = security_snapshot(raw, now()).expect("snapshot");
        assert_eq!(
            snapshot.firewall,
            FirewallState::Active("nftables".to_owned())
        );
        assert!(snapshot.has_fail2ban);
        assert_eq!(snapshot.jails[0].name, "sshd");
        assert_eq!(snapshot.jails[0].banned, vec!["192.0.2.10", "198.51.100.7"]);
        assert_eq!(snapshot.sshd.password_auth, Some(Switch::Off));
        assert_eq!(snapshot.sshd.login_grace_time, Some(120));
        assert_eq!(snapshot.sshd.client_alive_interval, Some(0));
        assert_eq!(snapshot.sshd.allow_tcp_forwarding, Some(Switch::On));
        assert_eq!(snapshot.sshd.max_startups.as_deref(), Some("10:30:100"));
        assert_eq!(
            snapshot.sshd.permit_root_login.as_deref(),
            Some("prohibit-password")
        );
        assert_eq!(snapshot.sshd.port, Some(22));
        assert_eq!(snapshot.ban_backend, Some(BanBackend::Fail2ban));
        assert_eq!(snapshot.bans.len(), 4);
        assert_eq!(snapshot.bans[2].source, "nftables");
        assert_eq!(snapshot.bans[2].expires.as_deref(), Some("23h58m1s"));
        assert_eq!(snapshot.hash_of("/etc/passwd"), Some("aaaa"));
        assert!(snapshot.is_root_view);
    }

    #[test]
    fn syslog_fixture_without_root_is_partial_but_parses() {
        let raw = include_str!("../../fixtures/security/syslog.txt");
        let snapshot = security_snapshot(raw, now()).expect("snapshot");
        assert_eq!(snapshot.firewall, FirewallState::Active("ufw".to_owned()));
        assert!(!snapshot.sshd.is_known());
        assert_eq!(snapshot.attackers[0].ip, "203.0.113.9");
        assert_eq!(snapshot.attackers[0].failures, 3);
        assert_eq!(snapshot.ban_backend, Some(BanBackend::Ufw));
        assert_eq!(snapshot.bans[0].ip, "203.0.113.9");
        assert!(!snapshot.is_root_view);
    }

    #[test]
    fn carry_slow_part_keeps_hardening_from_previous_snapshot() {
        let raw = include_str!("../../fixtures/security/server.txt");
        let previous = security_snapshot(raw, now()).expect("previous");
        let fast_only: String = raw.split("###sshd").next().expect("prefix").to_owned();
        let mut fresh = security_snapshot(&fast_only, now()).expect("fresh");
        assert!(!fresh.sshd.is_known());
        carry_slow_part(&mut fresh, &previous);
        assert_eq!(fresh.sshd, previous.sshd);
        assert_eq!(fresh.file_hashes, previous.file_hashes);
        assert_eq!(fresh.slow_collected_at, previous.slow_collected_at);
    }
}
