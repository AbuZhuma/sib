use chrono::{DateTime, Utc};

const SYSTEMD_TIME_FORMAT: &str = "%a %Y-%m-%d %H:%M:%S %z";

pub fn systemd_time(raw: &str) -> Option<DateTime<Utc>> {
    let fields: Vec<&str> = raw.split_whitespace().collect();
    if fields.len() < 4 {
        return None;
    }
    let zone = normalize_zone(fields[3]);
    let text = format!("{} {} {} {zone}", fields[0], fields[1], fields[2]);
    DateTime::parse_from_str(&text, SYSTEMD_TIME_FORMAT)
        .ok()
        .map(|t| t.with_timezone(&Utc))
}

fn normalize_zone(zone: &str) -> String {
    let is_numeric = zone.starts_with(['+', '-']) && zone[1..].chars().all(|c| c.is_ascii_digit());
    match (is_numeric, zone.len()) {
        (true, 3) => format!("{zone}00"),
        (true, _) => zone.to_owned(),
        (false, _) => "+0000".to_owned(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Timelike;

    #[test]
    fn systemd_time_with_short_offset_is_parsed() {
        let at = systemd_time("Mon 2026-09-14 12:30:00 +06").expect("time");
        assert_eq!(at.hour(), 6);
    }

    #[test]
    fn systemd_time_with_named_zone_falls_back_to_utc() {
        let at = systemd_time("Mon 2026-09-14 12:30:00 UTC").expect("time");
        assert_eq!(at.hour(), 12);
    }

    #[test]
    fn systemd_time_empty_is_none() {
        assert_eq!(systemd_time(""), None);
    }
}
