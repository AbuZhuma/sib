use sib_docgen::SectionId;

use super::{FORMAT_SECTION, Playbook};

const PROCESSES: Playbook = Playbook {
    sections: &[
        SectionId::Findings,
        SectionId::Processes,
        SectionId::Resources,
    ],
    task: "Look at the processes on the server: what is eating CPU and memory, whether there are zombies, \
suspicious or pointless processes (miners, unknown binaries from /tmp, duplicates). Compare with the \
load in the resources section.",
    format: FORMAT_SECTION,
};

const RESOURCES: Playbook = Playbook {
    sections: &[
        SectionId::Findings,
        SectionId::Resources,
        SectionId::Processes,
    ],
    task: "Look at the server resources from the current values and the hour and day summaries: CPU, \
memory, swap, disks, inodes, network. Look for trends (disk filling up, swap always in use, CPU \
spikes), memory pressure, errors and drops on the interfaces. Say how much headroom is left for the \
next few days.",
    format: FORMAT_SECTION,
};

const PORTS: Playbook = Playbook {
    sections: &[
        SectionId::Findings,
        SectionId::Ports,
        SectionId::Security,
        SectionId::Docker,
    ],
    task: "Look at the listening ports and the firewall: which services are open to the outside, which of \
them should not be public (databases, admin panels, debug ports), whether there are ports with no \
process, what is opened in the firewall without need, and what the firewall is missing.",
    format: FORMAT_SECTION,
};

const DOCKER: Playbook = Playbook {
    sections: &[SectionId::Findings, SectionId::Docker, SectionId::Resources],
    task: "Look at the containers: those that exited with a non-zero code, restarts, unhealthy ones, \
containers with no restart policy for services that should always run, ports published on 0.0.0.0, \
outdated images, leftovers (images, volumes). Take compose projects into account.",
    format: FORMAT_SECTION,
};

const SERVICES: Playbook = Playbook {
    sections: &[SectionId::Findings, SectionId::Services, SectionId::Logs],
    task: "Look at the systemd services: failed ones, ones that restart often, custom units and their \
state. Compare with the errors in the journal.",
    format: FORMAT_SECTION,
};

const LOGS: Playbook = Playbook {
    sections: &[
        SectionId::Findings,
        SectionId::Logs,
        SectionId::Services,
        SectionId::Resources,
    ],
    task: "Look at the system journal: the grouped repeating messages and the latest entries. Separate \
noise from real problems, explain the most frequent errors, say which of them are tied to failed \
services or to resources, and what to do to make them go away.",
    format: FORMAT_SECTION,
};

const USERS: Playbook = Playbook {
    sections: &[SectionId::Findings, SectionId::Users, SectionId::Security],
    task: "Look at the users: active sessions and where they come from, the latest logins, accounts with a \
shell and with sudo, the number of keys. Look for extra accounts with access, logins from unfamiliar \
addresses, root logins, and accounts with sudo but no keys.",
    format: FORMAT_SECTION,
};

const SECURITY: Playbook = Playbook {
    sections: &[
        SectionId::Findings,
        SectionId::Security,
        SectionId::Ports,
        SectionId::Users,
        SectionId::Updates,
    ],
    task: "Look at security: failed and unknown checks (and what is needed to run them), attacking \
addresses and bans, logins and sudo calls, security updates. Rank them by the real risk to this \
server, not by the order of the list.",
    format: FORMAT_SECTION,
};

const ANOMALIES: Playbook = Playbook {
    sections: &[
        SectionId::Findings,
        SectionId::Anomalies,
        SectionId::Ports,
        SectionId::Security,
    ],
    task: "Look at the network picture: connection states, packet and new connection rates, syn cookies \
and drops, the share of the top addresses, conntrack. Say whether there are signs of an attack or \
scanning, and which addresses look suspicious.",
    format: FORMAT_SECTION,
};

const DEPLOY: Playbook = Playbook {
    sections: &[SectionId::Findings, SectionId::Deploy, SectionId::Docker],
    task: "Look at the deploys: how often, how long, failures, unfinished ones, compose events. Look for \
unstable projects (frequent failures), deploys outside working hours, and stuck ones.",
    format: FORMAT_SECTION,
};

const GPU: Playbook = Playbook {
    sections: &[SectionId::Findings, SectionId::Gpu, SectionId::Processes],
    task: "Look at the GPU: load, memory, temperature, power, processes on the GPU. Look for overheating, \
memory filling up, and idle or foreign processes.",
    format: FORMAT_SECTION,
};

pub fn section_audit(key: &str) -> Option<Playbook> {
    let playbook = match key {
        "processes" => PROCESSES,
        "resources" => RESOURCES,
        "ports" => PORTS,
        "docker" => DOCKER,
        "services" => SERVICES,
        "logs" => LOGS,
        "users" => USERS,
        "security" => SECURITY,
        "anomalies" => ANOMALIES,
        "deploy" => DEPLOY,
        "gpu" => GPU,
        _ => return None,
    };
    Some(playbook)
}
