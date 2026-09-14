use chrono::{DateTime, Datelike, NaiveDateTime, Utc};

const ISO_FORMAT: &str = "%Y-%m-%dT%H:%M:%S%z";
const SYSLOG_FORMAT: &str = "%Y %b %d %H:%M:%S";

pub struct LogLine<'a> {
    pub at: DateTime<Utc>,
    pub identifier: &'a str,
    pub message: &'a str,
}

pub fn parse_line<'a>(line: &'a str, now: DateTime<Utc>) -> Option<LogLine<'a>> {
    let (header, message) = line.split_once(": ")?;
    let (at, rest) = split_timestamp(header, now)?;
    let mut fields = rest.split_whitespace();
    let _host = fields.next()?;
    let identifier = fields.next()?;
    let identifier = identifier.split('[').next().unwrap_or(identifier);
    Some(LogLine {
        at,
        identifier,
        message,
    })
}

fn split_timestamp(header: &str, now: DateTime<Utc>) -> Option<(DateTime<Utc>, &str)> {
    let header = header.trim_start();
    if header.starts_with(|c: char| c.is_ascii_digit()) {
        let (token, rest) = header.split_once(' ')?;
        let at = DateTime::parse_from_str(token, ISO_FORMAT).ok()?;
        return Some((at.with_timezone(&Utc), rest));
    }
    let mut parts = header.splitn(4, ' ');
    let month = parts.next()?;
    let day = parts.next()?;
    let time = parts.next()?;
    let rest = parts.next()?;
    let at = syslog_time(month, day, time, now)?;
    Some((at, rest))
}

fn syslog_time(month: &str, day: &str, time: &str, now: DateTime<Utc>) -> Option<DateTime<Utc>> {
    let candidate = |year: i32| {
        NaiveDateTime::parse_from_str(&format!("{year} {month} {day} {time}"), SYSLOG_FORMAT)
            .ok()
            .map(|naive| naive.and_utc())
    };
    let this_year = candidate(now.year())?;
    if this_year > now {
        return candidate(now.year() - 1);
    }
    Some(this_year)
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
    fn iso_line_with_offset_is_converted_to_utc() {
        let line = "2026-09-14T15:12:35+0600 fedora sshd[1234]: Failed password for root from 1.2.3.4 port 5 ssh2";
        let parsed = parse_line(line, now()).expect("line");
        assert_eq!(
            parsed.at,
            Utc.with_ymd_and_hms(2026, 9, 14, 9, 12, 35)
                .single()
                .expect("at")
        );
        assert_eq!(parsed.identifier, "sshd");
        assert!(parsed.message.starts_with("Failed password"));
    }

    #[test]
    fn syslog_line_uses_current_year() {
        let line = "Sep 14 10:00:01 host sshd-session[7]: Accepted publickey for u from 1.1.1.1 port 2 ssh2";
        let parsed = parse_line(line, now()).expect("line");
        assert_eq!(parsed.at.year(), 2026);
        assert_eq!(parsed.identifier, "sshd-session");
    }

    #[test]
    fn syslog_line_in_future_falls_back_to_previous_year() {
        let line = "Dec 30 10:00:01 host sudo[7]: x : COMMAND=/bin/true";
        let parsed = parse_line(line, now()).expect("line");
        assert_eq!(parsed.at.year(), 2025);
    }
}
