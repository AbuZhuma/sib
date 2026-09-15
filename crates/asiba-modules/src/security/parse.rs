use asiba_core::ModuleError;
use chrono::{DateTime, Utc};

use super::hardening;
use super::journal;
use super::model::{Ban, BanBackend, FirewallState, Jail, SecuritySnapshot, SshdSettings, Switch};
use crate::common::sections::Sections;

const FIREWALL_UNITS: [&str; 5] = [
    "firewalld",
    "ufw",
    "nftables",
    "iptables",
    "netfilter-persistent",
];
const JAIL_MARKER: &str = "@@ ";
const MAX_LOGINS: usize = 50;
const MAX_SUDO_CALLS: usize = 50;

pub fn security_snapshot(raw: &str, now: DateTime<Utc>) -> Result<SecuritySnapshot, ModuleError> {
    let sections = Sections::parse(raw);
    let units = sections.get_or_empty("units");
    if units.trim().is_empty() && sections.get("ssh").is_none() {
        return Err(ModuleError::Parse("пустой вывод security".to_owned()));
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
    })
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

fn jails(raw: &str) -> Vec<Jail> {
    let mut jails = Vec::new();
    for block in raw.split(JAIL_MARKER).skip(1) {
        let Some((name, body)) = block.split_once('\n') else {
            continue;
        };
        jails.push(Jail {
            name: name.trim().to_owned(),
            currently_failed: status_number(body, "Currently failed:"),
            total_banned: status_number(body, "Total banned:"),
            banned: status_value(body, "Banned IP list:")
                .split_whitespace()
                .map(str::to_owned)
                .collect(),
        });
    }
    jails
}

fn status_value<'a>(body: &'a str, label: &str) -> &'a str {
    body.lines()
        .find_map(|l| l.split_once(label))
        .map(|(_, v)| v.trim())
        .unwrap_or_default()
}

fn status_number(body: &str, label: &str) -> u64 {
    status_value(body, label).parse().unwrap_or(0)
}

fn jail_bans(jails: &[Jail]) -> Vec<Ban> {
    jails
        .iter()
        .flat_map(|jail| {
            jail.banned.iter().map(|ip| Ban {
                ip: ip.clone(),
                source: format!("fail2ban/{}", jail.name),
                expires: None,
            })
        })
        .collect()
}

fn nft_bans(raw: &str) -> Vec<Ban> {
    let Some(start) = raw.find("elements = {") else {
        return Vec::new();
    };
    let body = &raw[start + "elements = {".len()..];
    let body = body.split('}').next().unwrap_or_default();
    body.split(',')
        .map(str::trim)
        .filter(|e| !e.is_empty())
        .filter_map(|element| {
            let mut tokens = element.split_whitespace();
            let ip = tokens.next()?;
            let expires = element
                .split_once("expires ")
                .map(|(_, e)| e.split_whitespace().next().unwrap_or_default().to_owned());
            Some(Ban {
                ip: ip.to_owned(),
                source: "nftables".to_owned(),
                expires,
            })
        })
        .collect()
}

fn iptables_bans(raw: &str) -> Vec<Ban> {
    raw.lines()
        .filter(|l| l.starts_with("-A ASIBA") && l.contains("-j DROP"))
        .filter_map(|l| l.split_once("-s ").map(|(_, rest)| rest))
        .filter_map(|rest| rest.split_whitespace().next())
        .map(|ip| Ban {
            ip: ip
                .trim_end_matches("/32")
                .trim_end_matches("/128")
                .to_owned(),
            source: "iptables".to_owned(),
            expires: None,
        })
        .collect()
}

fn ufw_bans(raw: &str) -> Vec<Ban> {
    raw.lines()
        .filter(|l| l.contains("DENY"))
        .filter_map(|l| l.split_whitespace().last())
        .filter(|ip| ip.chars().next().is_some_and(|c| c.is_ascii_hexdigit()))
        .map(|ip| Ban {
            ip: ip.to_owned(),
            source: "ufw".to_owned(),
            expires: None,
        })
        .collect()
}

fn sshd_settings(raw: &str) -> SshdSettings {
    let mut settings = SshdSettings::default();
    for line in raw.lines() {
        let line = line.split_once(':').map(|(_, l)| l).unwrap_or(line);
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
    fn iptables_chain_lines_yield_bans() {
        let bans = iptables_bans("-N ASIBA\n-A ASIBA -s 10.0.0.5/32 -j DROP\n");
        assert_eq!(bans[0].ip, "10.0.0.5");
    }
}
