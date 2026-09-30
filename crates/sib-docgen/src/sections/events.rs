use sib_core::Event;

use super::alerts::severity_name;
use crate::section::{DocContext, Section, SectionId};
use crate::write::{blank, field, heading, list, local_time, table, truncate};

pub struct EventsSection;

const MAX_EVENTS: usize = 30;
const MESSAGE_CHARS: usize = 200;

fn events<'a>(ctx: &'a DocContext<'_>) -> Vec<&'a Event> {
    ctx.server
        .recent_events
        .iter()
        .rev()
        .take(MAX_EVENTS)
        .collect()
}

fn row(event: &Event) -> Vec<String> {
    vec![
        local_time(event.at),
        severity_name(event.severity).to_owned(),
        event.module.0.to_owned(),
        truncate(&event.message, MESSAGE_CHARS),
    ]
}

impl Section for EventsSection {
    fn id(&self) -> SectionId {
        SectionId::Events
    }

    fn is_available(&self, ctx: &DocContext<'_>) -> bool {
        !events(ctx).is_empty()
    }

    fn human(&self, out: &mut String, ctx: &DocContext<'_>) {
        heading(out, "Recent events");
        let rows: Vec<Vec<String>> = events(ctx).into_iter().map(row).collect();
        table(out, &["Time", "Level", "Module", "Message"], &rows);
    }

    fn llm(&self, out: &mut String, ctx: &DocContext<'_>) {
        heading(out, "Recent events");
        field(out, "events", "time | severity | module | message");
        let rows: Vec<Vec<String>> = events(ctx).into_iter().map(row).collect();
        list(out, "", &rows);
        blank(out);
    }
}
