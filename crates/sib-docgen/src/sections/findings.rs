use sib_core::{IncidentDraft, Severity};

use super::alerts::severity_name;
use crate::section::{DocContext, Section, SectionId};
use crate::write::{blank, field, heading, line};

pub struct FindingsSection;

fn severity_label(severity: Severity) -> &'static str {
    match severity {
        Severity::Critical => "критично",
        Severity::Warning => "внимание",
        Severity::Info => "к сведению",
    }
}

fn drafts(ctx: &DocContext<'_>) -> Vec<IncidentDraft> {
    let mut drafts = sib_incidents::detect(ctx.server, ctx.state);
    if !ctx.server.connection.is_online() {
        drafts.insert(
            0,
            IncidentDraft::new(
                sib_core::IncidentKind::Alert,
                Severity::Critical,
                "offline",
                "Сервер недоступен",
            ),
        );
    }
    drafts
}

impl Section for FindingsSection {
    fn id(&self) -> SectionId {
        SectionId::Findings
    }

    fn is_available(&self, _ctx: &DocContext<'_>) -> bool {
        true
    }

    fn human(&self, out: &mut String, ctx: &DocContext<'_>) {
        heading(out, "Что требует внимания");
        let drafts = drafts(ctx);
        if drafts.is_empty() {
            line(out, "Проблем не обнаружено.\n");
            return;
        }
        for draft in drafts {
            line(
                out,
                format!(
                    "- **{}** [{}] {}",
                    severity_label(draft.severity),
                    draft.kind.key(),
                    draft.summary
                ),
            );
        }
        blank(out);
    }

    fn llm(&self, out: &mut String, ctx: &DocContext<'_>) {
        heading(out, "Findings (detected by Sib)");
        let drafts = drafts(ctx);
        let critical = drafts
            .iter()
            .filter(|d| d.severity == Severity::Critical)
            .count();
        field(out, "total", drafts.len().to_string());
        field(out, "critical", critical.to_string());
        for draft in drafts {
            line(
                out,
                format!(
                    "- [{}] {}: {}",
                    severity_name(draft.severity),
                    draft.kind.key(),
                    draft.summary
                ),
            );
        }
        blank(out);
    }
}

#[cfg(test)]
mod tests {
    use sib_core::AppState;

    use super::*;
    use crate::test_support::server;

    #[test]
    fn findings_for_new_server_report_offline_only() {
        let state = AppState::default();
        let server = server();
        let drafts = drafts(&DocContext::new(&server, &state));
        assert_eq!(drafts.len(), 1);
        assert_eq!(drafts[0].subject, "offline");
    }
}
