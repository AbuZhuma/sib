use std::fmt::Write;

use chrono::{DateTime, Local, Utc};

pub const NONE: &str = "-";

pub fn line(out: &mut String, text: impl AsRef<str>) {
    let _ = writeln!(out, "{}", text.as_ref());
}

pub fn blank(out: &mut String) {
    out.push('\n');
}

pub fn heading(out: &mut String, title: &str) {
    let _ = writeln!(out, "## {title}\n");
}

pub fn subheading(out: &mut String, title: &str) {
    let _ = writeln!(out, "### {title}\n");
}

pub fn bullet(out: &mut String, label: &str, value: impl AsRef<str>) {
    let _ = writeln!(out, "- {label}: {}", value.as_ref());
}

pub fn field(out: &mut String, key: &str, value: impl AsRef<str>) {
    let _ = writeln!(out, "{key}: {}", value.as_ref());
}

pub fn table(out: &mut String, headers: &[&str], rows: &[Vec<String>]) {
    let _ = writeln!(out, "| {} |", headers.join(" | "));
    let _ = writeln!(out, "|{}", "---|".repeat(headers.len()));
    for row in rows {
        let cells: Vec<String> = row.iter().map(|c| c.replace('|', "/")).collect();
        let _ = writeln!(out, "| {} |", cells.join(" | "));
    }
    out.push('\n');
}

pub fn list(out: &mut String, prefix: &str, rows: &[Vec<String>]) {
    for row in rows {
        let _ = writeln!(out, "{prefix}{}", row.join(" | "));
    }
    out.push('\n');
}

pub fn more(out: &mut String, hidden: usize, label: &str) {
    if hidden > 0 {
        let _ = writeln!(out, "... +{hidden} {label}");
    }
}

pub fn bytes(value: u64) -> String {
    const UNITS: [&str; 5] = ["B", "KiB", "MiB", "GiB", "TiB"];
    let mut value = value as f64;
    let mut unit = 0;
    while value >= 1024.0 && unit < UNITS.len() - 1 {
        value /= 1024.0;
        unit += 1;
    }
    if unit == 0 {
        format!("{value:.0} {}", UNITS[unit])
    } else {
        format!("{value:.1} {}", UNITS[unit])
    }
}

pub fn bytes_per_second(value: f64) -> String {
    format!("{}/s", bytes(value.max(0.0) as u64))
}

pub fn percent(value: f64) -> String {
    format!("{value:.0}%")
}

pub fn local_time(at: DateTime<Utc>) -> String {
    at.with_timezone(&Local)
        .format("%Y-%m-%d %H:%M")
        .to_string()
}

pub fn utc_time(at: DateTime<Utc>) -> String {
    at.format("%Y-%m-%dT%H:%M:%SZ").to_string()
}

pub fn truncate(text: &str, max_chars: usize) -> String {
    if text.chars().count() <= max_chars {
        return text.to_owned();
    }
    let cut: String = text.chars().take(max_chars.saturating_sub(1)).collect();
    format!("{cut}…")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bytes_formats_units() {
        assert_eq!(bytes(512), "512 B");
        assert_eq!(bytes(1536), "1.5 KiB");
        assert_eq!(bytes(3 * 1024 * 1024 * 1024), "3.0 GiB");
    }

    #[test]
    fn truncate_adds_ellipsis_only_when_needed() {
        assert_eq!(truncate("abc", 5), "abc");
        assert_eq!(truncate("abcdefgh", 5), "abcd…");
    }

    #[test]
    fn table_escapes_pipes() {
        let mut out = String::new();
        table(&mut out, &["a", "b"], &[vec!["x|y".into(), "z".into()]]);
        assert_eq!(out, "| a | b |\n|---|---|\n| x/y | z |\n\n");
    }
}
