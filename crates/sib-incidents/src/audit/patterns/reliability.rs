use sib_modules::deploy::{self, DeployState, DeployStatus};
use sib_modules::docker::{self, DockerSnapshot};
use sib_modules::services::{self, ServicesSnapshot};

use crate::audit::context::{AuditContext, list_or};
use crate::audit::evidence::{EvidenceSource, TAB_DEPLOY, TAB_DOCKER, TAB_SERVICES};
use crate::audit::pattern::{Area, Pattern, Verdict, Weight};

const RESTARTS_WARN: u32 = 1;
const RESTARTS_FAIL: u32 = 10;
const RESTART_LOOP: u32 = 5;
const DANGLING_WARN: usize = 5;
const DANGLING_FAIL: usize = 30;
const NO_DATA: &str = "the module has not collected data";

const ADVICE_FAILED_UNITS: &str = "Open journalctl -u <unit> (the journal button on the Services tab), fix the cause and restart the unit.";
const ADVICE_RESTARTS: &str =
    "Check the unit journal. Frequent restarts mean the same error keeps happening.";
const ADVICE_CONTAINERS_DOWN: &str =
    "Look at docker logs (the logs button) and the exit code, then start the container.";
const ADVICE_UNHEALTHY: &str =
    "Check what the container depends on (database, network) and its logs.";
const ADVICE_RESTART_LOOP: &str = "The error shows up in the container logs at startup.";
const ADVICE_NO_POLICY: &str = "Set --restart unless-stopped or describe the container in compose.";
const ADVICE_DANGLING: &str = "Old layers take up disk space: docker image prune.";
const ADVICE_DEPLOYS: &str =
    "Open the Deploys tab: it shows the stage where the deploy stopped and the tail of the log.";

pub static PATTERNS: &[Pattern] = &[
    Pattern {
        id: "reliability.failed_units",
        area: Area::Reliability,
        subject: "Failed systemd units",
        description: "Units in the failed state. The service did not start, or it crashed and was not restarted.",
        weight: Weight::High,
        advice: ADVICE_FAILED_UNITS,
        evidence: EvidenceSource::Tab(TAB_SERVICES),
        evaluate: failed_units,
    },
    Pattern {
        id: "reliability.unit_restarts",
        area: Area::Reliability,
        subject: "Unit restarts",
        description: "The NRestarts counter: how many times systemd brought the unit back after a crash.",
        weight: Weight::Medium,
        advice: ADVICE_RESTARTS,
        evidence: EvidenceSource::Tab(TAB_SERVICES),
        evaluate: unit_restarts,
    },
    Pattern {
        id: "reliability.containers_down",
        area: Area::Reliability,
        subject: "Stopped containers",
        description: "Containers that should be running, by restart policy or because they exited with an error, but are not.",
        weight: Weight::High,
        advice: ADVICE_CONTAINERS_DOWN,
        evidence: EvidenceSource::Tab(TAB_DOCKER),
        evaluate: containers_down,
    },
    Pattern {
        id: "reliability.unhealthy",
        area: Area::Reliability,
        subject: "Container healthchecks",
        description: "Running containers whose healthcheck reports unhealthy.",
        weight: Weight::Medium,
        advice: ADVICE_UNHEALTHY,
        evidence: EvidenceSource::Tab(TAB_DOCKER),
        evaluate: unhealthy,
    },
    Pattern {
        id: "reliability.restart_loop",
        area: Area::Reliability,
        subject: "Container restart loop",
        description: "A container with a high RestartCount crashes right after starting and comes back again.",
        weight: Weight::Medium,
        advice: ADVICE_RESTART_LOOP,
        evidence: EvidenceSource::Tab(TAB_DOCKER),
        evaluate: restart_loop,
    },
    Pattern {
        id: "reliability.restart_policy",
        area: Area::Reliability,
        subject: "Container restart policy",
        description: "Containers without a restart policy will not come back after the server reboots.",
        weight: Weight::Low,
        advice: ADVICE_NO_POLICY,
        evidence: EvidenceSource::Tab(TAB_DOCKER),
        evaluate: restart_policy,
    },
    Pattern {
        id: "reliability.dangling_images",
        area: Area::Reliability,
        subject: "Dangling Docker images",
        description: "Images without a tag left over from builds and pulls. They take up disk space.",
        weight: Weight::Low,
        advice: ADVICE_DANGLING,
        evidence: EvidenceSource::Tab(TAB_DOCKER),
        evaluate: dangling_images,
    },
    Pattern {
        id: "reliability.deploys",
        area: Area::Reliability,
        subject: "Recent deploys",
        description: "Deploys with the failed status in the deploy module history.",
        weight: Weight::Medium,
        advice: ADVICE_DEPLOYS,
        evidence: EvidenceSource::Tab(TAB_DEPLOY),
        evaluate: deploys,
    },
];

fn failed_units(ctx: &AuditContext<'_>) -> Vec<Verdict> {
    let Some(snapshot) = ctx.data::<ServicesSnapshot>(services::ID) else {
        return Verdict::skipped(NO_DATA).single();
    };
    let failed: Vec<&str> = snapshot.failed().map(|u| u.name.as_str()).collect();
    Verdict::graded(
        !failed.is_empty(),
        false,
        list_or(&failed, "all units are fine"),
    )
    .single()
}

fn unit_restarts(ctx: &AuditContext<'_>) -> Vec<Verdict> {
    let Some(snapshot) = ctx.data::<ServicesSnapshot>(services::ID) else {
        return Verdict::skipped(NO_DATA).single();
    };
    let restarting: Vec<String> = snapshot
        .units
        .iter()
        .filter(|u| u.restarts >= RESTARTS_WARN)
        .map(|u| format!("{} ({})", u.name, u.restarts))
        .collect();
    let max_restarts = snapshot.units.iter().map(|u| u.restarts).max().unwrap_or(0);
    Verdict::graded(
        max_restarts >= RESTARTS_FAIL,
        !restarting.is_empty(),
        list_or(&restarting, "no restarts"),
    )
    .single()
}

fn should_run(policy: &str) -> bool {
    matches!(policy, "always" | "unless-stopped" | "on-failure")
}

fn containers_down(ctx: &AuditContext<'_>) -> Vec<Verdict> {
    let Some(snapshot) = ctx.data::<DockerSnapshot>(docker::ID) else {
        return Verdict::skipped(NO_DATA).single();
    };
    let down: Vec<String> = snapshot
        .containers
        .iter()
        .filter(|c| !c.is_running() && (c.exit_code != 0 || should_run(&c.restart_policy)))
        .map(|c| format!("{} (code {})", c.name, c.exit_code))
        .collect();
    Verdict::graded(!down.is_empty(), false, list_or(&down, "all are running")).single()
}

fn unhealthy(ctx: &AuditContext<'_>) -> Vec<Verdict> {
    let Some(snapshot) = ctx.data::<DockerSnapshot>(docker::ID) else {
        return Verdict::skipped(NO_DATA).single();
    };
    let unhealthy: Vec<&str> = snapshot
        .containers
        .iter()
        .filter(|c| c.is_running() && c.is_unhealthy())
        .map(|c| c.name.as_str())
        .collect();
    Verdict::graded(
        false,
        !unhealthy.is_empty(),
        list_or(&unhealthy, "none are unhealthy"),
    )
    .single()
}

fn restart_loop(ctx: &AuditContext<'_>) -> Vec<Verdict> {
    let Some(snapshot) = ctx.data::<DockerSnapshot>(docker::ID) else {
        return Verdict::skipped(NO_DATA).single();
    };
    let looping: Vec<String> = snapshot
        .containers
        .iter()
        .filter(|c| c.restart_count >= RESTART_LOOP)
        .map(|c| format!("{} ({})", c.name, c.restart_count))
        .collect();
    Verdict::graded(false, !looping.is_empty(), list_or(&looping, "no loops")).single()
}

fn restart_policy(ctx: &AuditContext<'_>) -> Vec<Verdict> {
    let Some(snapshot) = ctx.data::<DockerSnapshot>(docker::ID) else {
        return Verdict::skipped(NO_DATA).single();
    };
    let no_policy: Vec<&str> = snapshot
        .containers
        .iter()
        .filter(|c| c.is_running() && !should_run(&c.restart_policy))
        .map(|c| c.name.as_str())
        .collect();
    Verdict::graded(
        false,
        !no_policy.is_empty(),
        list_or(&no_policy, "all of them have one"),
    )
    .single()
}

fn dangling_images(ctx: &AuditContext<'_>) -> Vec<Verdict> {
    let Some(snapshot) = ctx.data::<DockerSnapshot>(docker::ID) else {
        return Verdict::skipped(NO_DATA).single();
    };
    let dangling = snapshot.images.iter().filter(|i| i.is_dangling()).count();
    Verdict::graded(
        dangling >= DANGLING_FAIL,
        dangling >= DANGLING_WARN,
        format!("{dangling} dangling images"),
    )
    .single()
}

fn deploys(ctx: &AuditContext<'_>) -> Vec<Verdict> {
    let Some(state) = ctx.data::<DeployState>(deploy::ID) else {
        return Verdict::skipped(NO_DATA).single();
    };
    let failed: Vec<&str> = state
        .snapshot
        .deploys
        .iter()
        .filter(|d| d.status == DeployStatus::Failed)
        .map(|d| d.project.as_str())
        .collect();
    Verdict::graded(
        !failed.is_empty(),
        false,
        list_or(&failed, "no failed deploys"),
    )
    .single()
}
