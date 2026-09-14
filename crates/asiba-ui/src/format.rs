use chrono::{DateTime, Local, Utc};

const UNITS: [&str; 5] = ["B", "KiB", "MiB", "GiB", "TiB"];

pub fn bytes(value: u64) -> String {
    let mut size = value as f64;
    let mut unit = 0;
    while size >= 1024.0 && unit < UNITS.len() - 1 {
        size /= 1024.0;
        unit += 1;
    }
    if unit == 0 {
        return format!("{value} {}", UNITS[0]);
    }
    format!("{size:.1} {}", UNITS[unit])
}

pub fn bytes_per_second(value: f64) -> String {
    format!("{}/s", bytes(value.max(0.0) as u64))
}

pub fn duration_short(seconds: f64) -> String {
    let secs = seconds.max(0.0) as u64;
    if secs >= 86_400 {
        return format!("{}д {}ч", secs / 86_400, (secs % 86_400) / 3_600);
    }
    if secs >= 3_600 {
        return format!("{}ч {}м", secs / 3_600, (secs % 3_600) / 60);
    }
    if secs >= 60 {
        return format!("{}м {}с", secs / 60, secs % 60);
    }
    format!("{secs}с")
}

pub fn clock(at: DateTime<Utc>) -> String {
    at.with_timezone(&Local).format("%H:%M:%S").to_string()
}

pub fn date_time(at: DateTime<Utc>) -> String {
    at.with_timezone(&Local)
        .format("%Y-%m-%d %H:%M")
        .to_string()
}

pub fn seconds_until(at: DateTime<Utc>) -> i64 {
    (at - Utc::now()).num_seconds().max(0)
}

pub fn signed_seconds(value: i64) -> String {
    if value == 0 {
        return "0 с".to_owned();
    }
    format!("{value:+} с")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bytes_small_value_has_no_decimals() {
        assert_eq!(bytes(512), "512 B");
    }

    #[test]
    fn bytes_gib_value_has_one_decimal() {
        assert_eq!(bytes(8_143_268 * 1024), "7.8 GiB");
    }

    #[test]
    fn signed_seconds_positive_has_plus() {
        assert_eq!(signed_seconds(3), "+3 с");
    }
}
