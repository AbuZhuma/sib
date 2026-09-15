use asiba_core::{Incident, IncidentKind, ModuleId, QueryRequest};
use asiba_docgen::SectionId;
use asiba_modules::{docker, services};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Playbook {
    pub sections: &'static [SectionId],
    pub task: &'static str,
}

const FULL: Playbook = Playbook {
    sections: &SectionId::ALL,
    task: "Perform a full operational and security audit of this server using only the data above. \
Group the output as: Summary (2-3 sentences), Critical (must fix now), Recommended (should fix soon), \
Notes (minor or informational). For every item name the evidence line it comes from and give a concrete \
command or configuration change. If Findings is empty and nothing looks wrong, say so briefly.",
};

const CONTAINER: Playbook = Playbook {
    sections: &[
        SectionId::Findings,
        SectionId::Docker,
        SectionId::Projects,
        SectionId::Resources,
        SectionId::Deploy,
        SectionId::Events,
    ],
    task: "The container '{subject}' is down or unhealthy. Using its logs and the Docker/project state, \
explain the most likely root cause (crash, misconfiguration, missing dependency, resource exhaustion, \
image problem) with the exact log lines that support it, then give concrete steps to fix it and to \
prevent it from happening again.",
};

const UNIT: Playbook = Playbook {
    sections: &[
        SectionId::Findings,
        SectionId::Services,
        SectionId::Logs,
        SectionId::Resources,
        SectionId::Projects,
        SectionId::Events,
    ],
    task: "The systemd unit '{subject}' has failed. Using its journal and the system state, explain the \
root cause with the supporting log lines, then give the exact commands to diagnose further and to fix it.",
};

const DEPLOY: Playbook = Playbook {
    sections: &[
        SectionId::Findings,
        SectionId::Deploy,
        SectionId::Projects,
        SectionId::Docker,
        SectionId::Services,
        SectionId::Logs,
    ],
    task: "The deploy '{subject}' failed. From the deploy log tail and the state of containers and \
services, identify the failing step and its cause, then list what to check and how to fix it.",
};

const ATTACK: Playbook = Playbook {
    sections: &[
        SectionId::Findings,
        SectionId::Anomalies,
        SectionId::Security,
        SectionId::Ports,
        SectionId::Resources,
        SectionId::Events,
    ],
    task: "A network anomaly or attack sign was detected: '{subject}'. Decide whether this looks like an \
attack (DDoS, scan, brute force) or legitimate load, explain why using the counters and peers, and give \
concrete mitigation: which IPs or ranges to block, which firewall or sysctl rules to apply, what to monitor.",
};

const SECURITY: Playbook = Playbook {
    sections: &[
        SectionId::Findings,
        SectionId::Security,
        SectionId::Users,
        SectionId::Ports,
        SectionId::Updates,
    ],
    task: "A security check failed: '{subject}'. Explain the risk in this server's context and give the \
exact configuration change or command to fix it, plus how to verify the fix.",
};

const RESOURCES: Playbook = Playbook {
    sections: &[
        SectionId::Findings,
        SectionId::Resources,
        SectionId::Processes,
        SectionId::Docker,
        SectionId::Services,
        SectionId::Logs,
    ],
    task: "A resource problem was detected: '{subject}'. Identify what consumes the resource using the \
process, container and service data, say whether it is a leak, a spike or normal growth, and give \
concrete steps to relieve it now and to prevent it.",
};

pub fn full_audit() -> Playbook {
    FULL
}

pub fn playbook(kind: IncidentKind) -> Playbook {
    match kind {
        IncidentKind::ContainerDown => CONTAINER,
        IncidentKind::UnitFailed => UNIT,
        IncidentKind::DeployFailed => DEPLOY,
        IncidentKind::Anomaly | IncidentKind::BruteForce => ATTACK,
        IncidentKind::SecurityCheck | IncidentKind::Updates => SECURITY,
        IncidentKind::Alert
        | IncidentKind::DiskFull
        | IncidentKind::Memory
        | IncidentKind::ModuleError
        | IncidentKind::Clock => RESOURCES,
    }
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
    fn every_playbook_starts_with_findings() {
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
}
