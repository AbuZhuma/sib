const UNITS: [(&str, f64); 12] = [
    ("kib", 1024.0),
    ("mib", 1024.0 * 1024.0),
    ("gib", 1024.0 * 1024.0 * 1024.0),
    ("tib", 1024.0 * 1024.0 * 1024.0 * 1024.0),
    ("kb", 1000.0),
    ("mb", 1_000_000.0),
    ("gb", 1_000_000_000.0),
    ("tb", 1_000_000_000_000.0),
    ("k", 1000.0),
    ("m", 1_000_000.0),
    ("g", 1_000_000_000.0),
    ("b", 1.0),
];

pub fn parse_bytes(raw: &str) -> Option<u64> {
    let text = raw.trim().to_lowercase().replace(' ', "");
    let digits_end = text.find(|c: char| !(c.is_ascii_digit() || c == '.'))?;
    let (number, unit) = text.split_at(digits_end);
    let value: f64 = number.parse().ok()?;
    let factor = UNITS
        .iter()
        .find(|(name, _)| *name == unit)
        .map(|(_, f)| *f)?;
    Some((value * factor).round() as u64)
}

pub fn parse_pair(raw: &str) -> Option<(u64, u64)> {
    let (left, right) = raw.split_once('/')?;
    Some((parse_bytes(left)?, parse_bytes(right)?))
}

pub fn parse_percent(raw: &str) -> Option<f64> {
    raw.trim().trim_end_matches('%').parse().ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_bytes_handles_decimal_and_binary_units() {
        assert_eq!(parse_bytes("38.87MB"), Some(38_870_000));
        assert_eq!(parse_bytes("16.4GiB"), Some(17_609_365_914));
        assert_eq!(parse_bytes("927 MB"), Some(927_000_000));
        assert_eq!(parse_bytes("0B"), Some(0));
    }

    #[test]
    fn parse_pair_splits_on_slash() {
        assert_eq!(parse_pair("618B / 1.382kB"), Some((618, 1382)));
    }

    #[test]
    fn parse_bytes_garbage_is_none() {
        assert_eq!(parse_bytes("n/a"), None);
    }
}
