use sib_core::Incident;

use crate::playbook::Playbook;

pub const SYSTEM_PROMPT: &str = "You are a senior Linux operations and security engineer. \
You are given a structured data dump collected by the Sib monitoring tool, and a task. Rules: \
use only the data in the dump, do not invent numbers, names, log lines or commands that are not there; \
if data is missing, say what is missing and how to get it; give concrete commands and configuration \
snippets for the distribution named in the System section (dnf or apt, systemctl, firewalld, ufw or \
nftables, whatever the server has); for every conclusion point at the data line it comes from; tell a \
cause from an effect; do not give generic advice such as \"update the system\" when the data shows no \
problem; be short, this text goes on a screen, it is not an article. Answer in English and keep the \
names of commands, paths, units, packages and metrics as they are. Levels: critical means active harm \
or an open hole; recommended means fix it in the next few days; note means a small thing.";

pub fn build_user(context: &str, playbook: &Playbook, incident: Option<&Incident>) -> String {
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
    out.push_str("## Data\n\n");
    out.push_str(context);
    out.push_str("\n\n## Task\n\n");
    out.push_str(&playbook.task.replace(
        "{subject}",
        incident.map(|i| i.subject.as_str()).unwrap_or("server"),
    ));
    out.push_str("\n\n");
    out.push_str(playbook.format);
    out
}

#[cfg(test)]
mod tests {
    use chrono::Utc;
    use sib_core::{IncidentDraft, IncidentKind, ServerId, Severity};

    use super::*;
    use crate::playbook;

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
        let text = build_user(
            "data",
            &playbook::playbook(IncidentKind::ContainerDown),
            Some(&incident),
        );
        assert!(text.contains("kind: container_down"));
        assert!(text.contains("severity: critical"));
        assert!(text.contains("evidence: exit code 137"));
        assert!(text.contains("Container \"web\""));
        assert!(text.ends_with(playbook::FORMAT_INCIDENT));
    }
}
