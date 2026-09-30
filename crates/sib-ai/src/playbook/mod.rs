mod fleet;
mod incidents;
mod sections;

use sib_core::{Incident, IncidentKind, ModuleId, QueryRequest};
use sib_docgen::SectionId;
use sib_modules::{docker, services};

pub use fleet::fleet_audit;
pub use sections::section_audit;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Playbook {
    pub sections: &'static [SectionId],
    pub task: &'static str,
    pub format: &'static str,
}

pub const FORMAT_AUDIT: &str = "Answer format (Markdown, only these headings, skip empty sections):\n\
## Summary\nTwo or three sentences: the overall state and the main risk.\n\
## Critical\nWhat needs action now. For each item: the fact (a data line), the risk, the exact command or configuration change.\n\
## Recommended\nWhat to fix soon, in the same format.\n\
## Notes\nSmall things and observations, one line each.";

pub const FORMAT_INCIDENT: &str = "Answer format (Markdown, only these headings):\n\
## Cause\nOne or two sentences: the most likely cause and how sure you are (high, medium, low).\n\
## Evidence\nThe data or log lines the answer is based on (quote them exactly, up to 8 lines).\n\
## What to do now\nStep by step: the diagnostic and fix commands in the order to run them.\n\
## How to prevent it\nSettings, monitoring, limits, briefly.";

pub const FORMAT_SECTION: &str = "Answer format (Markdown, only these headings, short, this is a block on screen next to the data):\n\
## State\nOne or two sentences to the point.\n\
## What looks wrong\nFacts from the data with why they matter. If there is nothing, one line: nothing important.\n\
## Recommendations\nUp to five concrete actions with commands. If all is well, what is worth keeping an eye on.";

pub const FORMAT_FLEET: &str = "Answer format (Markdown, only these headings):\n\
## Overall state\nTwo or three sentences about the whole fleet.\n\
## Need attention\nServers in order of urgency: name, what is wrong, what to do first. Critical incidents always come above the rest.\n\
## First steps\nThe three most useful things to do today.";

const FULL: Playbook = Playbook {
    sections: &SectionId::ALL,
    task: "Do a full operations and security audit of this server. Start with the Findings section: \
those are problems the program already found, so explain and rank them instead of repeating them. Then \
look for what the program does not see: mismatches between sections (an open port with no process, a \
container restarting while resources are free, a rise in journal errors), settings that are outdated \
or risky, signs of leaks and of slow degradation in the hour and day summaries. If the server is fine, \
say so briefly and do not invent problems.",
    format: FORMAT_AUDIT,
};

pub fn full_audit() -> Playbook {
    FULL
}

pub fn playbook(kind: IncidentKind) -> Playbook {
    incidents::for_kind(kind)
}

pub fn queries(incident: &Incident) -> Vec<(ModuleId, QueryRequest)> {
    match incident.kind {
        IncidentKind::ContainerDown => vec![(
            docker::ID,
            QueryRequest::new(docker::QUERY_LOGS, incident.subject.clone()),
        )],
        IncidentKind::UnitFailed => vec![(
            services::ID,
            QueryRequest::new(services::QUERY_JOURNAL, incident.subject.clone()),
        )],
        _ => Vec::new(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_incident_playbook_starts_with_findings() {
        let kinds = [
            IncidentKind::ContainerDown,
            IncidentKind::UnitFailed,
            IncidentKind::DeployFailed,
            IncidentKind::Anomaly,
            IncidentKind::SecurityCheck,
            IncidentKind::DiskFull,
        ];
        for kind in kinds {
            assert_eq!(playbook(kind).sections[0], SectionId::Findings, "{kind:?}");
        }
        assert_eq!(full_audit().sections[0], SectionId::Findings);
    }

    #[test]
    fn section_audit_exists_for_every_visible_section() {
        for key in [
            "processes",
            "resources",
            "ports",
            "docker",
            "services",
            "logs",
            "users",
            "security",
            "anomalies",
            "deploy",
            "gpu",
        ] {
            assert!(section_audit(key).is_some(), "{key}");
        }
        assert!(section_audit("nonexistent").is_none());
    }
}
