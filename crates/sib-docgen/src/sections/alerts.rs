use sib_core::{Alert, Severity};

use crate::section::{DocContext, Section, SectionId};
use crate::write::{NONE, blank, field, heading, list, local_time, table};

pub struct AlertsSection;

const MAX_RESOLVED: usize = 10;

pub fn severity_name(severity: Severity) -> &'static str {
    match severity {
        Severity::Info => "info",
        Severity::Warning => "warning",
        Severity::Critical => "critical",
    }
}

fn active<'a>(ctx: &'a DocContext<'_>) -> Vec<&'a Alert> {
    ctx.state
        .active_alerts()
        .filter(|a| a.server == ctx.server.spec.id)
        .collect()
}

fn resolved<'a>(ctx: &'a DocContext<'_>) -> Vec<&'a Alert> {
    ctx.state
        .alerts
        .iter()
        .rev()
        .filter(|a| a.server == ctx.server.spec.id && a.resolved_at.is_some())
        .take(MAX_RESOLVED)
        .collect()
}

fn row(alert: &Alert) -> Vec<String> {
    let state = if alert.acknowledged {
        "acknowledged"
    } else {
        ""
    };
    vec![
        severity_name(alert.severity).to_owned(),
        alert.rule_name.clone(),
        alert.message.clone(),
        local_time(alert.started_at),
        alert
            .resolved_at
            .map(local_time)
            .unwrap_or_else(|| NONE.to_owned()),
        state.to_owned(),
    ]
}

impl Section for AlertsSection {
    fn id(&self) -> SectionId {
        SectionId::Alerts
    }

    fn is_available(&self, ctx: &DocContext<'_>) -> bool {
        !active(ctx).is_empty() || !resolved(ctx).is_empty()
    }

    fn human(&self, out: &mut String, ctx: &DocContext<'_>) {
        heading(out, "Алерты");
        let headers = [
            "Уровень",
            "Правило",
            "Сообщение",
            "Начало",
            "Конец",
            "Статус",
        ];
        let active: Vec<Vec<String>> = active(ctx).into_iter().map(row).collect();
        if active.is_empty() {
            out.push_str("Активных алертов нет.\n\n");
        } else {
            table(out, &headers, &active);
        }
        let resolved: Vec<Vec<String>> = resolved(ctx).into_iter().map(row).collect();
        if !resolved.is_empty() {
            out.push_str("Недавно закрытые:\n\n");
            table(out, &headers, &resolved);
        }
    }

    fn llm(&self, out: &mut String, ctx: &DocContext<'_>) {
        heading(out, "Alerts");
        let active: Vec<Vec<String>> = active(ctx).into_iter().map(row).collect();
        field(out, "active_count", active.len().to_string());
        field(
            out,
            "active",
            "severity | rule | message | started | resolved | state",
        );
        list(out, "", &active);
        let resolved: Vec<Vec<String>> = resolved(ctx).into_iter().map(row).collect();
        if !resolved.is_empty() {
            field(
                out,
                "recently_resolved",
                "severity | rule | message | started | resolved | state",
            );
            list(out, "", &resolved);
        }
        blank(out);
    }
}
