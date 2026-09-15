use asiba_core::Incident;

pub const SYSTEM_PROMPT: &str = "You are a senior Linux operations and security engineer reviewing a \
server for its administrator. You receive a structured data dump collected by a monitoring tool (Asiba) \
and a task. Rules: use only the data provided, never invent numbers, names or log lines; when data is \
missing say what is missing and how to get it; prefer concrete commands and config snippets for the \
detected distribution; be specific about which line of the data supports each claim; keep the answer \
compact with Markdown headings and bullet lists; answer in Russian (keep commands, paths, \
unit and package names as they are). Severity words: critical = active \
harm or exposure, recommended = should be fixed soon, note = minor.";

pub fn build_user(context: &str, task: &str, incident: Option<&Incident>) -> String {
    let mut out = String::new();
    if let Some(incident) = incident {
        out.push_str("## Incident\n\n");
        out.push_str(&format!("kind: {}\n", incident.kind.key()));
        out.push_str(&format!("severity: {:?}\n", incident.severity).to_lowercase());
        out.push_str(&format!("subject: {}\n", incident.subject));
        out.push_str(&format!("summary: {}\n", incident.summary));
        out.push_str(&format!(
            "started_at: {}\n",
            incident.started_at.format("%Y-%m-%dT%H:%M:%SZ")
        ));
        for line in &incident.evidence {
            out.push_str(&format!("evidence: {line}\n"));
        }
        out.push('\n');
    }
    out.push_str("## Server data\n\n");
    out.push_str(context);
    out.push_str("\n\n## Task\n\n");
    out.push_str(&task.replace(
        "{subject}",
        incident.map(|i| i.subject.as_str()).unwrap_or("server"),
    ));
    out
}

#[cfg(test)]
mod tests {
    use asiba_core::{IncidentDraft, IncidentKind, ServerId, Severity};
    use chrono::Utc;

    use super::*;

    #[test]
    fn build_user_substitutes_subject_and_lists_evidence() {
        let draft = IncidentDraft::new(
            IncidentKind::ContainerDown,
            Severity::Critical,
            "web",
            "container web is exited",
        )
        .evidence(["exit code 137".to_owned()]);
        let incident = Incident::open(1, ServerId::parse("neo").expect("id"), draft, Utc::now());
        let text = build_user("data", "Fix '{subject}'.", Some(&incident));
        assert!(text.contains("kind: container_down"));
        assert!(text.contains("severity: critical"));
        assert!(text.contains("evidence: exit code 137"));
        assert!(text.ends_with("Fix 'web'."));
    }
}
