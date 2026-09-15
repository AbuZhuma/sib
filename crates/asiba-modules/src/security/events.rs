use asiba_core::{Event, Severity};
use chrono::{DateTime, Utc};

use super::ID;
use super::model::SecuritySnapshot;

const WATCHED_FILES: [&str; 3] = ["/etc/passwd", "/etc/group", "/etc/sudoers"];

pub fn between(
    previous: &SecuritySnapshot,
    current: &SecuritySnapshot,
    previous_taken_at: DateTime<Utc>,
) -> Vec<Event> {
    let mut events = Vec::new();
    brute_force(previous, current, &mut events);
    logins(previous, current, previous_taken_at, &mut events);
    file_changes(previous, current, &mut events);
    events
}

fn brute_force(previous: &SecuritySnapshot, current: &SecuritySnapshot, events: &mut Vec<Event>) {
    for attacker in current.attackers.iter().filter(|a| a.is_brute_force()) {
        let was_active = previous
            .attackers
            .iter()
            .any(|a| a.ip == attacker.ip && a.is_brute_force());
        if was_active {
            continue;
        }
        events.push(Event::new(
            ID,
            Severity::Warning,
            format!(
                "брутфорс SSH с {}: {} попыток за 10 мин ({})",
                attacker.ip,
                attacker.recent_failures,
                attacker.users_label()
            ),
        ));
    }
}

fn logins(
    previous: &SecuritySnapshot,
    current: &SecuritySnapshot,
    previous_taken_at: DateTime<Utc>,
    events: &mut Vec<Event>,
) {
    for login in current.logins_since(previous_taken_at) {
        if previous.logins.iter().any(|l| l.same(login)) {
            continue;
        }
        let is_new_ip = !current.has_login_from(&login.from, previous_taken_at);
        let severity = if login.is_root() || is_new_ip {
            Severity::Warning
        } else {
            Severity::Info
        };
        let suffix = if is_new_ip {
            " - новый адрес"
        } else {
            ""
        };
        events.push(Event::new(
            ID,
            severity,
            format!(
                "вход {} с {} ({}){suffix}",
                login.user, login.from, login.method
            ),
        ));
    }
}

fn file_changes(previous: &SecuritySnapshot, current: &SecuritySnapshot, events: &mut Vec<Event>) {
    for path in WATCHED_FILES {
        let (Some(before), Some(after)) = (previous.hash_of(path), current.hash_of(path)) else {
            continue;
        };
        if before != after {
            events.push(Event::new(
                ID,
                Severity::Critical,
                format!("изменён {path}"),
            ));
        }
    }
    for (path, hash) in &current.file_hashes {
        if !path.starts_with("/etc/sudoers.d/") {
            continue;
        }
        match previous.hash_of(path) {
            None => events.push(Event::new(
                ID,
                Severity::Critical,
                format!("новый файл {path}"),
            )),
            Some(before) if before != hash => {
                events.push(Event::new(
                    ID,
                    Severity::Critical,
                    format!("изменён {path}"),
                ));
            }
            _ => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::security::model::{Attacker, FirewallState, Login, SshdSettings};
    use chrono::TimeZone;

    fn at(minute: u32) -> DateTime<Utc> {
        Utc.with_ymd_and_hms(2026, 9, 14, 12, minute, 0)
            .single()
            .expect("at")
    }

    fn empty() -> SecuritySnapshot {
        SecuritySnapshot {
            attackers: Vec::new(),
            failed_logins: 0,
            logins: Vec::new(),
            sudo_calls: Vec::new(),
            jails: Vec::new(),
            has_fail2ban: false,
            sshd: SshdSettings::default(),
            firewall: FirewallState::Unknown,
            file_hashes: vec![("/etc/passwd".to_owned(), "a".to_owned())],
            bans: Vec::new(),
            ban_backend: None,
            is_root_view: false,
            hardening: Default::default(),
        }
    }

    fn attacker(ip: &str, recent: u64) -> Attacker {
        Attacker {
            ip: ip.to_owned(),
            failures: recent,
            recent_failures: recent,
            users: vec!["root".to_owned()],
            first_at: at(0),
            last_at: at(5),
        }
    }

    fn login(user: &str, from: &str, minute: u32) -> Login {
        Login {
            at: at(minute),
            user: user.to_owned(),
            from: from.to_owned(),
            method: "publickey".to_owned(),
        }
    }

    #[test]
    fn brute_force_crossing_threshold_raises_warning_once() {
        let previous = SecuritySnapshot {
            attackers: vec![attacker("1.1.1.1", 5)],
            ..empty()
        };
        let current = SecuritySnapshot {
            attackers: vec![attacker("1.1.1.1", 12)],
            ..empty()
        };
        let events = between(&previous, &current, at(1));
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].severity, Severity::Warning);
        assert!(between(&current, &current, at(1)).is_empty());
    }

    #[test]
    fn login_from_new_address_is_warning_and_known_address_is_info() {
        let previous = SecuritySnapshot {
            logins: vec![login("deploy", "9.9.9.9", 0)],
            ..empty()
        };
        let current = SecuritySnapshot {
            logins: vec![
                login("deploy", "8.8.8.8", 3),
                login("deploy", "9.9.9.9", 2),
                login("deploy", "9.9.9.9", 0),
            ],
            ..empty()
        };
        let events = between(&previous, &current, at(1));
        assert_eq!(events.len(), 2);
        assert_eq!(events[0].severity, Severity::Warning);
        assert_eq!(events[1].severity, Severity::Info);
    }

    #[test]
    fn changed_passwd_hash_is_critical() {
        let current = SecuritySnapshot {
            file_hashes: vec![("/etc/passwd".to_owned(), "b".to_owned())],
            ..empty()
        };
        let events = between(&empty(), &current, at(1));
        assert_eq!(events[0].severity, Severity::Critical);
        assert_eq!(events[0].message, "изменён /etc/passwd");
    }
}
