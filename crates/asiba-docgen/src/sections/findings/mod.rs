mod infrastructure;
mod security;
mod system;

use asiba_core::Severity;

use super::alerts::severity_name;
use crate::section::{DocContext, Section, SectionId};
use crate::write::{blank, field, heading, line};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Finding {
    pub severity: Severity,
    pub area: &'static str,
    pub text: String,
}

impl Finding {
    pub fn new(severity: Severity, area: &'static str, text: impl Into<String>) -> Self {
        Self {
            severity,
            area,
            text: text.into(),
        }
    }
}

pub fn collect_findings(ctx: &DocContext<'_>) -> Vec<Finding> {
    let mut findings = Vec::new();
    system::collect(ctx, &mut findings);
    security::collect(ctx, &mut findings);
    infrastructure::collect(ctx, &mut findings);
    findings.sort_by_key(|f| std::cmp::Reverse(f.severity));
    findings
}

pub struct FindingsSection;

fn severity_label(severity: Severity) -> &'static str {
    match severity {
        Severity::Critical => "критично",
        Severity::Warning => "внимание",
        Severity::Info => "к сведению",
    }
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
        let findings = collect_findings(ctx);
        if findings.is_empty() {
            line(out, "Проблем не обнаружено.\n");
            return;
        }
        for finding in findings {
            line(
                out,
                format!(
                    "- **{}** [{}] {}",
                    severity_label(finding.severity),
                    finding.area,
                    finding.text
                ),
            );
        }
        blank(out);
    }

    fn llm(&self, out: &mut String, ctx: &DocContext<'_>) {
        heading(out, "Findings (detected by Asiba)");
        let findings = collect_findings(ctx);
        let critical = findings
            .iter()
            .filter(|f| f.severity == Severity::Critical)
            .count();
        field(out, "total", findings.len().to_string());
        field(out, "critical", critical.to_string());
        for finding in findings {
            line(
                out,
                format!(
                    "- [{}] {}: {}",
                    severity_name(finding.severity),
                    finding.area,
                    finding.text
                ),
            );
        }
        blank(out);
    }
}

#[cfg(test)]
mod tests {
    use asiba_core::AppState;

    use super::*;
    use crate::test_support::server;

    #[test]
    fn findings_for_new_server_report_offline_only() {
        let state = AppState::default();
        let server = server();
        let findings = collect_findings(&DocContext::new(&server, &state));
        assert_eq!(findings.len(), 1);
        assert_eq!(findings[0].area, "connection");
        assert_eq!(findings[0].severity, Severity::Critical);
    }
}
