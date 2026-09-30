use chrono::DateTime;
use serde::Deserialize;

use super::model::LogEntry;

const MICROS: i64 = 1_000_000;

#[derive(Deserialize)]
struct JournalLine {
    #[serde(rename = "__REALTIME_TIMESTAMP")]
    timestamp: String,
    #[serde(rename = "PRIORITY")]
    priority: Option<String>,
    #[serde(rename = "_SYSTEMD_UNIT")]
    unit: Option<String>,
    #[serde(rename = "SYSLOG_IDENTIFIER")]
    identifier: Option<String>,
    #[serde(rename = "_PID")]
    pid: Option<String>,
    #[serde(rename = "MESSAGE")]
    message: Option<serde_json::Value>,
    #[serde(rename = "__CURSOR")]
    cursor: String,
}

pub fn entries(raw: &str) -> Vec<LogEntry> {
    raw.lines()
        .filter_map(|line| serde_json::from_str::<JournalLine>(line).ok())
        .filter_map(to_entry)
        .collect()
}

fn to_entry(line: JournalLine) -> Option<LogEntry> {
    let micros: i64 = line.timestamp.parse().ok()?;
    let at = DateTime::from_timestamp(micros / MICROS, ((micros % MICROS) * 1000) as u32)?;
    let message = match line.message {
        Some(serde_json::Value::String(text)) => text,
        Some(serde_json::Value::Array(bytes)) => bytes_to_string(&bytes),
        _ => String::new(),
    };
    Some(LogEntry {
        at,
        priority: line.priority.and_then(|p| p.parse().ok()).unwrap_or(6),
        unit: line.unit,
        identifier: line.identifier,
        pid: line.pid.and_then(|p| p.parse().ok()),
        message,
        cursor: line.cursor,
    })
}

fn bytes_to_string(values: &[serde_json::Value]) -> String {
    let bytes: Vec<u8> = values
        .iter()
        .filter_map(|v| v.as_u64())
        .map(|v| v as u8)
        .collect();
    String::from_utf8_lossy(&bytes).into_owned()
}

#[cfg(test)]
mod tests {
    use super::*;

    const JOURNAL: &str = include_str!("../../fixtures/logs/journal.jsonl");

    #[test]
    fn fixture_parses_entries_with_priority_and_cursor() {
        let parsed = entries(JOURNAL);
        assert_eq!(parsed.len(), 3);
        assert_eq!(parsed[0].priority, 4);
        assert_eq!(parsed[0].source(), "pipewire");
        assert_eq!(parsed[1].priority, 3);
        assert!(parsed[2].cursor.starts_with("s=c474"));
        assert_eq!(parsed[2].message, "binary\u{fffd}");
    }

    #[test]
    fn garbage_lines_are_skipped() {
        assert!(entries("not json\n{}\n").is_empty());
    }
}
