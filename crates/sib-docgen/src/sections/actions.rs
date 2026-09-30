use sib_core::ActionRecord;

use crate::section::{DocContext, Section, SectionId};
use crate::write::{blank, field, heading, list, local_time, table};

pub struct ActionsSection;

const MAX_ACTIONS: usize = 15;

fn records<'a>(ctx: &'a DocContext<'_>) -> Vec<&'a ActionRecord> {
    ctx.state
        .actions
        .iter()
        .rev()
        .filter(|a| a.server == ctx.server.spec.id)
        .take(MAX_ACTIONS)
        .collect()
}

fn row(record: &ActionRecord) -> Vec<String> {
    let result = if record.is_success { "ok" } else { "FAILED" };
    let target = match &record.argument {
        Some(argument) => format!("{} ({argument})", record.target),
        None => record.target.clone(),
    };
    vec![
        local_time(record.at),
        format!("{}.{}", record.module, record.kind),
        target,
        result.to_owned(),
        record.message.clone(),
    ]
}

impl Section for ActionsSection {
    fn id(&self) -> SectionId {
        SectionId::Actions
    }

    fn is_available(&self, ctx: &DocContext<'_>) -> bool {
        !records(ctx).is_empty()
    }

    fn human(&self, out: &mut String, ctx: &DocContext<'_>) {
        heading(out, "Журнал действий");
        let rows: Vec<Vec<String>> = records(ctx).into_iter().map(row).collect();
        table(
            out,
            &["Время", "Действие", "Цель", "Результат", "Сообщение"],
            &rows,
        );
    }

    fn llm(&self, out: &mut String, ctx: &DocContext<'_>) {
        heading(out, "Operator actions");
        field(out, "actions", "time | action | target | result | message");
        let rows: Vec<Vec<String>> = records(ctx).into_iter().map(row).collect();
        list(out, "", &rows);
        blank(out);
    }
}
