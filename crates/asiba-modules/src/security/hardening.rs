use super::model::{Hardening, MacStatus};
use crate::common::sections::Sections;

pub const SYSCTL_KEYS: [&str; 7] = [
    "net.ipv4.tcp_syncookies",
    "net.ipv4.conf.all.rp_filter",
    "kernel.randomize_va_space",
    "net.ipv4.ip_forward",
    "net.ipv4.icmp_echo_ignore_broadcasts",
    "net.ipv4.conf.all.accept_redirects",
    "net.ipv4.conf.all.send_redirects",
];

pub fn parse(sections: &Sections<'_>, units: &str) -> Hardening {
    Hardening {
        sysctl: sysctl(sections.get_or_empty("sysctl")),
        mac: mac(sections.get_or_empty("mac")),
        ntp_synced: yes_no(sections.get_or_empty("ntp")),
        extra_uid0: lines(sections.get_or_empty("uid0")),
        empty_passwords: sections.get("shadow").map(lines),
        sudo_nopasswd: sections
            .get("nopasswd")
            .and_then(|raw| raw.trim().parse().ok()),
        writable_keys: lines(sections.get_or_empty("keyperms")),
        world_writable_etc: lines(sections.get_or_empty("wwfiles")),
        risky_ports: risky_ports(sections.get_or_empty("risky")),
        auto_updates: unit_active(units, "unattended-upgrades")
            || unit_active(units, "dnf-automatic.timer")
            || unit_active(units, "dnf-automatic-install.timer"),
        auditd: unit_active(units, "auditd"),
    }
}

fn unit_active(units: &str, unit: &str) -> bool {
    units
        .lines()
        .filter_map(|l| l.split_once(' '))
        .any(|(name, state)| name == unit && state.trim() == "active")
}

fn sysctl(raw: &str) -> std::collections::BTreeMap<String, String> {
    raw.lines()
        .filter_map(|l| l.split_once('='))
        .map(|(k, v)| (k.trim().to_owned(), v.trim().to_owned()))
        .collect()
}

fn mac(raw: &str) -> Option<MacStatus> {
    let text = raw.trim();
    if text.is_empty() {
        return None;
    }
    let lower = text.to_ascii_lowercase();
    Some(if lower.contains("enforcing") {
        MacStatus::SelinuxEnforcing
    } else if lower.contains("permissive") {
        MacStatus::SelinuxPermissive
    } else if lower.contains("apparmor") {
        MacStatus::AppArmor
    } else if lower.contains("disabled") {
        MacStatus::Disabled
    } else {
        MacStatus::Unknown
    })
}

fn yes_no(raw: &str) -> Option<bool> {
    match raw.trim() {
        "yes" => Some(true),
        "no" => Some(false),
        _ => None,
    }
}

fn lines(raw: &str) -> Vec<String> {
    raw.lines()
        .map(str::trim)
        .filter(|l| !l.is_empty())
        .map(str::to_owned)
        .collect()
}

fn risky_ports(raw: &str) -> Vec<u16> {
    let mut ports: Vec<u16> = raw
        .lines()
        .filter_map(|l| l.trim().rsplit(':').next())
        .filter_map(|p| p.parse().ok())
        .collect();
    ports.sort_unstable();
    ports.dedup();
    ports
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hardening_sections_are_parsed() {
        let raw = "###units\nauditd active\nunattended-upgrades active\n###sysctl\nnet.ipv4.tcp_syncookies = 1\nkernel.randomize_va_space = 2\n###mac\nEnforcing\n###ntp\nyes\n###uid0\ntoor\n###shadow\n###nopasswd\n2\n###keyperms\n###wwfiles\n/etc/x\n###risky\n0.0.0.0:23\n[::]:2375\n";
        let sections = Sections::parse(raw);
        let hardening = parse(&sections, sections.get_or_empty("units"));
        assert_eq!(hardening.sysctl("net.ipv4.tcp_syncookies"), Some("1"));
        assert_eq!(hardening.mac, Some(MacStatus::SelinuxEnforcing));
        assert_eq!(hardening.ntp_synced, Some(true));
        assert_eq!(hardening.extra_uid0, vec!["toor"]);
        assert_eq!(hardening.empty_passwords, Some(Vec::new()));
        assert_eq!(hardening.sudo_nopasswd, Some(2));
        assert_eq!(hardening.world_writable_etc, vec!["/etc/x"]);
        assert_eq!(hardening.risky_ports, vec![23, 2375]);
        assert!(hardening.auto_updates);
        assert!(hardening.auditd);
    }

    #[test]
    fn missing_shadow_section_is_unknown() {
        let sections = Sections::parse("###units\n");
        let hardening = parse(&sections, "");
        assert_eq!(hardening.empty_passwords, None);
        assert_eq!(hardening.mac, None);
    }
}
