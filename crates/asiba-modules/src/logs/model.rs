use std::collections::BTreeMap;

use chrono::{DateTime, Duration, Utc};

const MAX_ENTRIES: usize = 2000;
const RETENTION: Duration = Duration::hours(24);
const SHORT_MESSAGE: usize = 160;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LogEntry {
    pub at: DateTime<Utc>,
    pub priority: u8,
    pub unit: Option<String>,
    pub identifier: Option<String>,
    pub pid: Option<u32>,
    pub message: String,
    pub cursor: String,
}

impl LogEntry {
    pub fn source(&self) -> &str {
        self.identifier
            .as_deref()
            .or(self.unit.as_deref())
            .unwrap_or("?")
    }

    pub fn message_short(&self) -> String {
        let line = self.message.lines().next().unwrap_or_default();
        if line.chars().count() <= SHORT_MESSAGE {
            return line.to_owned();
        }
        let cut: String = line.chars().take(SHORT_MESSAGE).collect();
        format!("{cut}…")
    }

    pub fn priority_label(&self) -> &'static str {
        match self.priority {
            0 => "emerg",
            1 => "alert",
            2 => "crit",
            3 => "err",
            4 => "warning",
            5 => "notice",
            6 => "info",
            _ => "debug",
        }
    }

    pub fn group_key(&self) -> String {
        let normalized: String = self
            .message_short()
            .chars()
            .map(|c| if c.is_ascii_digit() { '#' } else { c })
            .collect();
        format!("{}|{normalized}", self.source())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LogGroup {
    pub source: String,
    pub message: String,
    pub priority: u8,
    pub count: usize,
    pub last_at: DateTime<Utc>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LogsSnapshot {
    pub entries: Vec<LogEntry>,
    pub fresh: Vec<LogEntry>,
    pub cursor: Option<String>,
}

impl LogsSnapshot {
    pub fn count_since(&self, window: Duration, max_priority: u8) -> usize {
        let from = Utc::now() - window;
        self.entries
            .iter()
            .filter(|e| e.at >= from && e.priority <= max_priority)
            .count()
    }

    pub fn rate_per_minute(&self, max_priority: u8) -> f64 {
        self.count_since(Duration::minutes(5), max_priority) as f64 / 5.0
    }

    pub fn groups(&self) -> Vec<LogGroup> {
        let mut groups: BTreeMap<String, LogGroup> = BTreeMap::new();
        for entry in &self.entries {
            let group = groups.entry(entry.group_key()).or_insert_with(|| LogGroup {
                source: entry.source().to_owned(),
                message: entry.message_short(),
                priority: entry.priority,
                count: 0,
                last_at: entry.at,
            });
            group.count += 1;
            group.last_at = group.last_at.max(entry.at);
            group.priority = group.priority.min(entry.priority);
        }
        let mut list: Vec<LogGroup> = groups.into_values().collect();
        list.sort_by_key(|g| std::cmp::Reverse(g.last_at));
        list
    }
}

pub fn merge(previous: Option<&LogsSnapshot>, fresh: Vec<LogEntry>) -> LogsSnapshot {
    let cutoff = Utc::now() - RETENTION;
    let mut entries: Vec<LogEntry> = previous
        .map(|p| {
            p.entries
                .iter()
                .filter(|e| e.at >= cutoff)
                .cloned()
                .collect()
        })
        .unwrap_or_default();
    entries.extend(fresh.iter().cloned());
    if entries.len() > MAX_ENTRIES {
        let overflow = entries.len() - MAX_ENTRIES;
        entries.drain(..overflow);
    }
    let cursor = fresh
        .last()
        .map(|e| e.cursor.clone())
        .or_else(|| previous.and_then(|p| p.cursor.clone()));
    LogsSnapshot {
        entries,
        fresh,
        cursor,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn entry(message: &str, cursor: &str) -> LogEntry {
        LogEntry {
            at: Utc::now(),
            priority: 3,
            unit: Some("app.service".into()),
            identifier: Some("app".into()),
            pid: Some(1),
            message: message.into(),
            cursor: cursor.into(),
        }
    }

    #[test]
    fn merge_keeps_previous_entries_and_updates_cursor() {
        let first = merge(None, vec![entry("a", "c1")]);
        let second = merge(Some(&first), vec![entry("b", "c2")]);
        assert_eq!(second.entries.len(), 2);
        assert_eq!(second.cursor.as_deref(), Some("c2"));
    }

    #[test]
    fn merge_without_fresh_keeps_cursor() {
        let first = merge(None, vec![entry("a", "c1")]);
        let second = merge(Some(&first), vec![]);
        assert_eq!(second.cursor.as_deref(), Some("c1"));
    }

    #[test]
    fn groups_collapse_messages_differing_in_digits() {
        let snapshot = merge(
            None,
            vec![
                entry("timeout after 12s", "1"),
                entry("timeout after 30s", "2"),
            ],
        );
        let groups = snapshot.groups();
        assert_eq!(groups.len(), 1);
        assert_eq!(groups[0].count, 2);
    }
}
