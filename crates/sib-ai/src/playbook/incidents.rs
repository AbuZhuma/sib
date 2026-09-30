use sib_core::IncidentKind;
use sib_docgen::SectionId;

use super::{FORMAT_INCIDENT, Playbook};

const CONTAINER: Playbook = Playbook {
    sections: &[
        SectionId::Findings,
        SectionId::Docker,
        SectionId::Resources,
        SectionId::Deploy,
        SectionId::Events,
    ],
    task: "Container \"{subject}\" is stopped, crashing or unhealthy. From its logs (the container log \
section, if there is one), the state of Docker and of the project, work out the cause: the application \
crashed, a configuration or environment variable is wrong, a dependency is down (database, network, \
volume), resources ran out, or the image is broken. Tell a cause from an effect: if the logs show a \
first error before a cascade, that one is the real cause.",
    format: FORMAT_INCIDENT,
};

const UNIT: Playbook = Playbook {
    sections: &[
        SectionId::Findings,
        SectionId::Services,
        SectionId::Logs,
        SectionId::Resources,
        SectionId::Events,
    ],
    task: "The systemd unit \"{subject}\" is failed. From its journal (the journal section, if there is \
one), the service list and the system journal, work out the cause: a start error (ExecStart, \
permissions, a missing file, a busy port), a crashed process, a limit that was hit, or a dependency. \
Take the restart counter and the result field into account.",
    format: FORMAT_INCIDENT,
};

const DEPLOY: Playbook = Playbook {
    sections: &[
        SectionId::Findings,
        SectionId::Deploy,
        SectionId::Docker,
        SectionId::Services,
        SectionId::Logs,
    ],
    task: "The deploy \"{subject}\" failed. From the tail of the deploy log, the stages and the state of \
the containers and services, work out at which step and why it failed (build, migrations, start, \
health check, an unreachable registry or repository), and what to check before running it again.",
    format: FORMAT_INCIDENT,
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
    task: "A network attack sign or brute force was found: \"{subject}\". From the counters, the address \
list and the state of the defenses, decide whether this is an attack (DDoS, scanning, password \
guessing) or normal load, and say what the answer is based on. Give concrete steps: which IPs or \
ranges to block and with what on this server (fail2ban, nftables, iptables, ufw or firewalld, pick \
what is there), which sysctl and sshd settings to apply, and what to watch next.",
    format: FORMAT_INCIDENT,
};

const SECURITY: Playbook = Playbook {
    sections: &[
        SectionId::Findings,
        SectionId::Security,
        SectionId::Users,
        SectionId::Ports,
        SectionId::Updates,
    ],
    task: "A security check failed: \"{subject}\". Explain the risk for this particular server, taking its \
open ports, users and environment into account, and give the exact configuration change or command \
that fixes it plus a way to check that the fix took effect.",
    format: FORMAT_INCIDENT,
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
    task: "A resource problem: \"{subject}\". From the hour and day summaries, the top processes, the \
containers and the services, work out what is using the resource and what kind of problem it is: a \
leak (steady growth), a spike (a peak and a return) or normal growth. Give steps to take the pressure \
off now (what to restart, what to clean up, what to limit) and steps for later (limits, rotation, \
alerts).",
    format: FORMAT_INCIDENT,
};

pub fn for_kind(kind: IncidentKind) -> Playbook {
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
