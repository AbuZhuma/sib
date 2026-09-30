use std::collections::BTreeMap;

use chrono::{DateTime, Utc};

use super::model::{Attacker, Login, SudoCall, brute_force_since};
use crate::common::log_time::{self, LogLine};

const SSH_IDENTIFIERS: [&str; 2] = ["sshd", "sshd-session"];
const FAILURE_PREFIXES: [&str; 3] = [
    "Failed password for",
    "Failed publickey for",
    "Invalid user",
];
const ALREADY_COUNTED: [&str; 2] = [
    "Failed password for invalid user",
    "Failed publickey for invalid user",
];
const CLOSED_PREAUTH: &str = "Connection closed by authenticating user ";
const SUDO_FAILURE_MARKERS: [&str; 3] = [
    "incorrect password",
    "a password is required",
    "command not allowed",
];

pub struct SshActivity {
    pub attackers: Vec<Attacker>,
    pub failed_logins: u64,
    pub logins: Vec<Login>,
}

pub fn ssh_activity(raw: &str, now: DateTime<Utc>) -> SshActivity {
    let recent_since = brute_force_since(now);
    let mut attackers: BTreeMap<String, Attacker> = BTreeMap::new();
    let mut logins = Vec::new();
    let mut failed_logins = 0;
    for line in raw.lines().filter_map(|l| log_time::parse_line(l, now)) {
        if !SSH_IDENTIFIERS.contains(&line.identifier) {
            continue;
        }
        if let Some((user, ip)) = failure(&line) {
            failed_logins += 1;
            record_failure(&mut attackers, ip, user, line.at, recent_since);
        } else if let Some(login) = accepted(&line) {
            logins.push(login);
        }
    }
    let mut attackers: Vec<Attacker> = attackers.into_values().collect();
    attackers.sort_by_key(|a| std::cmp::Reverse(a.failures));
    logins.reverse();
    SshActivity {
        attackers,
        failed_logins,
        logins,
    }
}

fn failure<'a>(line: &LogLine<'a>) -> Option<(&'a str, &'a str)> {
    let message = line.message;
    if let Some(rest) = message.strip_prefix(CLOSED_PREAUTH) {
        let mut tokens = rest.split_whitespace();
        return Some((tokens.next()?, tokens.next()?));
    }
    if ALREADY_COUNTED.iter().any(|p| message.starts_with(p))
        || !FAILURE_PREFIXES.iter().any(|p| message.starts_with(p))
    {
        return None;
    }
    let (before_from, after_from) = message.split_once(" from ")?;
    let ip = after_from.split_whitespace().next()?;
    let user = before_from
        .trim_end()
        .rsplit(' ')
        .next()
        .filter(|u| !u.is_empty())?;
    Some((user, ip))
}

fn accepted(line: &LogLine<'_>) -> Option<Login> {
    let rest = line.message.strip_prefix("Accepted ")?;
    let (method, rest) = rest.split_once(" for ")?;
    let (user, rest) = rest.split_once(" from ")?;
    let from = rest.split_whitespace().next()?;
    Some(Login {
        at: line.at,
        user: user.to_owned(),
        from: from.to_owned(),
        method: method.to_owned(),
    })
}

fn record_failure(
    attackers: &mut BTreeMap<String, Attacker>,
    ip: &str,
    user: &str,
    at: DateTime<Utc>,
    recent_since: DateTime<Utc>,
) {
    let attacker = attackers.entry(ip.to_owned()).or_insert_with(|| Attacker {
        ip: ip.to_owned(),
        failures: 0,
        recent_failures: 0,
        users: Vec::new(),
        first_at: at,
        last_at: at,
    });
    attacker.failures += 1;
    if at >= recent_since {
        attacker.recent_failures += 1;
    }
    attacker.first_at = attacker.first_at.min(at);
    attacker.last_at = attacker.last_at.max(at);
    if !attacker.users.iter().any(|u| u == user) {
        attacker.users.push(user.to_owned());
    }
}

pub fn sudo_calls(raw: &str, now: DateTime<Utc>) -> Vec<SudoCall> {
    let mut calls: Vec<SudoCall> = raw
        .lines()
        .filter_map(|l| log_time::parse_line(l, now))
        .filter(|l| l.identifier == "sudo")
        .filter_map(|l| sudo_call(&l))
        .collect();
    calls.reverse();
    calls
}

fn sudo_call(line: &LogLine<'_>) -> Option<SudoCall> {
    let message = line.message.trim();
    let (user, details) = message.split_once(" : ")?;
    let command = details
        .split(" ; ")
        .find_map(|part| part.trim().strip_prefix("COMMAND="))?;
    let target_user = details
        .split(" ; ")
        .find_map(|part| part.trim().strip_prefix("USER="))
        .unwrap_or("root");
    let is_success = !SUDO_FAILURE_MARKERS.iter().any(|m| details.contains(m));
    Some(SudoCall {
        at: line.at,
        user: user.trim().to_owned(),
        target_user: target_user.to_owned(),
        command: command.to_owned(),
        is_success,
    })
}
