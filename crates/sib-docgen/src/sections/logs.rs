use chrono::Duration;
use sib_modules::logs::{self, LogsSnapshot};

use crate::section::{DocContext, Section, SectionId};
use crate::write::{blank, bullet, field, heading, list, local_time, table, truncate};

pub struct LogsSection;

const MAX_GROUPS: usize = 15;
const MAX_FRESH: usize = 15;
const MESSAGE_CHARS: usize = 160;
const HOUR: Duration = Duration::hours(1);
const PRIORITY_ERROR: u8 = 3;
const PRIORITY_WARNING: u8 = 4;

fn snapshot<'a>(ctx: &'a DocContext<'_>) -> Option<&'a LogsSnapshot> {
    ctx.server.data::<LogsSnapshot>(logs::ID)
}

fn group_rows(snapshot: &LogsSnapshot) -> Vec<Vec<String>> {
    let mut groups = snapshot.groups();
    groups.sort_by_key(|g| std::cmp::Reverse(g.count));
    groups
        .iter()
        .take(MAX_GROUPS)
        .map(|g| {
            vec![
                g.count.to_string(),
                g.source.clone(),
                priority_name(g.priority).to_owned(),
                local_time(g.last_at),
                truncate(&g.message, MESSAGE_CHARS),
            ]
        })
        .collect()
}

fn fresh_rows(snapshot: &LogsSnapshot) -> Vec<Vec<String>> {
    snapshot
        .fresh
        .iter()
        .rev()
        .take(MAX_FRESH)
        .map(|e| {
            vec![
                local_time(e.at),
                priority_name(e.priority).to_owned(),
                e.source().to_owned(),
                truncate(&e.message, MESSAGE_CHARS),
            ]
        })
        .collect()
}

fn priority_name(priority: u8) -> &'static str {
    match priority {
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

impl Section for LogsSection {
    fn id(&self) -> SectionId {
        SectionId::Logs
    }

    fn is_available(&self, ctx: &DocContext<'_>) -> bool {
        snapshot(ctx).is_some()
    }

    fn human(&self, out: &mut String, ctx: &DocContext<'_>) {
        let Some(snapshot) = snapshot(ctx) else {
            return;
        };
        heading(out, "Journal");
        bullet(
            out,
            "Errors in an hour",
            snapshot.count_since(HOUR, PRIORITY_ERROR).to_string(),
        );
        bullet(
            out,
            "Warnings in an hour",
            snapshot.count_since(HOUR, PRIORITY_WARNING).to_string(),
        );
        bullet(
            out,
            "Errors per minute",
            format!("{:.2}", snapshot.rate_per_minute(PRIORITY_ERROR)),
        );
        blank(out);
        table(
            out,
            &["Repeats", "Source", "Level", "Last", "Message"],
            &group_rows(snapshot),
        );
    }

    fn llm(&self, out: &mut String, ctx: &DocContext<'_>) {
        let Some(snapshot) = snapshot(ctx) else {
            return;
        };
        heading(out, "System log (journal)");
        field(
            out,
            "errors_last_hour",
            snapshot.count_since(HOUR, PRIORITY_ERROR).to_string(),
        );
        field(
            out,
            "warnings_last_hour",
            snapshot.count_since(HOUR, PRIORITY_WARNING).to_string(),
        );
        field(
            out,
            "top_messages",
            "count | source | priority | last_seen | message",
        );
        list(out, "", &group_rows(snapshot));
        field(out, "latest_entries", "time | priority | source | message");
        list(out, "", &fresh_rows(snapshot));
        blank(out);
    }
}
