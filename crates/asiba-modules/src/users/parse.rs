use asiba_core::ModuleError;

use super::model::{Account, LoginRecord, Session, UsersSnapshot};
use crate::common::sections::Sections;

const NOLOGIN_SHELLS: [&str; 4] = [
    "/sbin/nologin",
    "/usr/sbin/nologin",
    "/bin/false",
    "/usr/bin/false",
];

pub fn users_snapshot(raw: &str) -> Result<UsersSnapshot, ModuleError> {
    let sections = Sections::parse(raw);
    let passwd = sections.get_or_empty("passwd");
    if passwd.trim().is_empty() {
        return Err(ModuleError::Parse("пустой getent passwd".to_owned()));
    }
    let sudoers = sudoers(sections.get_or_empty("sudoers"));
    Ok(UsersSnapshot {
        sessions: sections
            .get_or_empty("who")
            .lines()
            .filter_map(who_line)
            .collect(),
        logins: sections
            .get_or_empty("last")
            .lines()
            .filter_map(last_line)
            .collect(),
        accounts: passwd
            .lines()
            .filter_map(|l| account_line(l, &sudoers))
            .collect(),
        authorized_keys: sections
            .get_or_empty("keys")
            .lines()
            .filter_map(keys_line)
            .collect(),
    })
}

fn who_line(line: &str) -> Option<Session> {
    let fields: Vec<&str> = line.split_whitespace().collect();
    if fields.len() < 4 {
        return None;
    }
    let from = fields.get(4).map(|f| f.trim_matches(['(', ')']).to_owned());
    Some(Session {
        user: fields[0].to_owned(),
        tty: fields[1].to_owned(),
        since: format!("{} {}", fields[2], fields[3]),
        from,
    })
}

fn last_line(line: &str) -> Option<LoginRecord> {
    let fields: Vec<&str> = line.split_whitespace().collect();
    if fields.len() < 8 || fields[0] == "wtmp" || fields[0] == "reboot" {
        return None;
    }
    let has_host = fields[2].contains('.') || fields[2].contains(':') || fields[2] == "local";
    let (from, rest) = if has_host {
        (fields[2], &fields[3..])
    } else {
        ("", &fields[2..])
    };
    Some(LoginRecord {
        user: fields[0].to_owned(),
        tty: fields[1].to_owned(),
        from: from.to_owned(),
        when: rest.iter().take(5).copied().collect::<Vec<_>>().join(" "),
        still_logged_in: line.contains("still logged in"),
    })
}

fn account_line(line: &str, sudoers: &[String]) -> Option<Account> {
    let fields: Vec<&str> = line.split(':').collect();
    if fields.len() < 7 || NOLOGIN_SHELLS.contains(&fields[6]) {
        return None;
    }
    let uid = fields[2].parse().ok()?;
    let name = fields[0].to_owned();
    let is_sudoer = uid == 0 || sudoers.contains(&name);
    Some(Account {
        name,
        uid,
        home: fields[5].to_owned(),
        shell: fields[6].to_owned(),
        is_sudoer,
    })
}

fn sudoers(raw: &str) -> Vec<String> {
    raw.lines()
        .filter_map(|line| line.split(':').nth(3))
        .flat_map(|members| members.split(','))
        .map(str::trim)
        .filter(|m| !m.is_empty())
        .map(str::to_owned)
        .collect()
}

fn keys_line(line: &str) -> Option<(String, u32)> {
    let (home, count) = line.rsplit_once(' ')?;
    Some((home.to_owned(), count.trim().parse().ok()?))
}

#[cfg(test)]
mod tests {
    use super::*;

    const SERVER: &str = include_str!("../../fixtures/users/server.txt");

    #[test]
    fn fixture_parses_sessions_accounts_and_keys() {
        let snapshot = users_snapshot(SERVER).expect("parse");
        assert_eq!(snapshot.sessions.len(), 2);
        assert_eq!(snapshot.sessions[1].from.as_deref(), Some("203.0.113.5"));
        let names: Vec<&str> = snapshot.accounts.iter().map(|a| a.name.as_str()).collect();
        assert_eq!(names, vec!["root", "deploy"]);
        assert!(snapshot.accounts[1].is_sudoer);
        assert_eq!(
            snapshot.authorized_keys,
            vec![("/root".to_owned(), 2), ("/home/deploy".to_owned(), 1)]
        );
    }

    #[test]
    fn fixture_parses_last_records_skipping_reboot() {
        let snapshot = users_snapshot(SERVER).expect("parse");
        assert_eq!(snapshot.logins.len(), 2);
        assert_eq!(snapshot.logins[0].from, "203.0.113.5");
        assert!(snapshot.logins[0].still_logged_in);
    }
}
