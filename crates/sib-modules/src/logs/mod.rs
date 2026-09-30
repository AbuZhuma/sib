mod model;
mod parse;

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use sib_core::transport::shell_quote;
use sib_core::{
    Availability, CollectContext, Event, Module, ModuleError, ModuleId, ModuleSettings, Sample,
    Schedule, Severity, Snapshot, Transport,
};

pub use model::{LogEntry, LogGroup, LogsSnapshot};

use crate::common::detect;

pub const ID: ModuleId = ModuleId("logs");
pub const KEY_WARNINGS_PER_MIN: &str = "logs.warnings_per_min";
pub const KEY_ERRORS_PER_MIN: &str = "logs.errors_per_min";

const INITIAL_LINES: u32 = 300;
const OLDER_LINES: u32 = 300;
const MICROS_PER_SECOND: i64 = 1_000_000;
const INITIAL_SINCE: &str = "-1h";
const BURST_THRESHOLD: usize = 30;
const CRITICAL_PRIORITY: u8 = 2;

pub struct LogsModule;

fn initial_command() -> String {
    format!(
        "journalctl -p warning -o json --no-pager -q --since {INITIAL_SINCE} -n {INITIAL_LINES}"
    )
}

fn older_command(before: DateTime<Utc>) -> String {
    let micros = before.timestamp_micros() - 1;
    format!(
        "journalctl -p warning -o json --no-pager -q --until=@{}.{:06} -n {OLDER_LINES}",
        micros.div_euclid(MICROS_PER_SECOND),
        micros.rem_euclid(MICROS_PER_SECOND)
    )
}

fn after_cursor_command(cursor: &str) -> String {
    format!(
        "journalctl -p warning -o json --no-pager -q --after-cursor={}",
        shell_quote(cursor)
    )
}

#[async_trait]
impl Module for LogsModule {
    fn id(&self) -> ModuleId {
        ID
    }

    fn title(&self) -> &'static str {
        "Logs"
    }

    fn schedule(&self) -> Schedule {
        Schedule::Normal
    }

    async fn detect(
        &self,
        transport: &dyn Transport,
        _settings: &ModuleSettings,
    ) -> Result<Availability, ModuleError> {
        let probe = detect::require(transport, "command -v journalctl", "no journalctl").await?;
        if !probe.is_usable() {
            return Ok(probe);
        }
        let probe = transport
            .exec("journalctl -q -n 1 -o cat --system 2>&1 | head -c 200")
            .await?;
        if probe.stdout.contains("No journal files") || probe.stdout.contains("permission") {
            return Ok(Availability::partial(
                "the system journal (needs the systemd-journal group)",
            ));
        }
        Ok(Availability::Available)
    }

    async fn collect(
        &self,
        transport: &dyn Transport,
        context: &CollectContext,
    ) -> Result<Snapshot, ModuleError> {
        let previous = context.previous::<LogsSnapshot>().map(|(p, _)| p);
        let command = match previous.and_then(|p| p.cursor.as_deref()) {
            Some(cursor) => after_cursor_command(cursor),
            None => initial_command(),
        };
        let output = transport.exec(&command).await?;
        let fresh = parse::entries(&output.stdout);
        let snapshot = model::merge(previous, fresh);
        let events = events_for(&snapshot, previous.is_some());
        let samples = vec![
            Sample::new(KEY_WARNINGS_PER_MIN, snapshot.rate_per_minute(4)),
            Sample::new(KEY_ERRORS_PER_MIN, snapshot.rate_per_minute(3)),
        ];
        Ok(Snapshot::new(snapshot)
            .with_samples(samples)
            .with_events(events))
    }

    async fn backfill(
        &self,
        transport: &dyn Transport,
        context: &CollectContext,
    ) -> Result<Snapshot, ModuleError> {
        backfill_older(transport, context).await
    }
}

async fn backfill_older(
    transport: &dyn Transport,
    context: &CollectContext,
) -> Result<Snapshot, ModuleError> {
    let previous = context
        .previous
        .as_ref()
        .and_then(|s| s.downcast::<LogsSnapshot>())
        .ok_or(ModuleError::UnsupportedBackfill)?;
    let Some(oldest) = previous.entries.first() else {
        return Err(ModuleError::UnsupportedBackfill);
    };
    let output = transport.exec(&older_command(oldest.at)).await?;
    let older = parse::entries(&output.stdout);
    Ok(Snapshot::new(model::prepend_older(previous, older)))
}

fn events_for(snapshot: &LogsSnapshot, has_previous: bool) -> Vec<Event> {
    if !has_previous {
        return Vec::new();
    }
    let mut events = Vec::new();
    for entry in snapshot
        .fresh
        .iter()
        .filter(|e| e.priority <= CRITICAL_PRIORITY)
    {
        let message = format!("[{}] {}", entry.source(), entry.message_short());
        events.push(Event::new(ID, Severity::Critical, message));
    }
    if snapshot.fresh.len() >= BURST_THRESHOLD {
        let message = format!(
            "burst of journal errors: {} in one cycle",
            snapshot.fresh.len()
        );
        events.push(Event::new(ID, Severity::Warning, message));
    }
    events
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn older_command_ends_one_microsecond_before_oldest_entry() {
        let at = DateTime::from_timestamp(1_700_000_000, 0).unwrap_or_default();
        let command = older_command(at);
        assert!(command.contains("--until=@1699999999.999999"));
        assert!(command.ends_with("-n 300"));
    }
}
