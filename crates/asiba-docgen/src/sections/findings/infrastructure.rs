use asiba_core::Severity;
use asiba_modules::deploy::{self, DeployState, DeployStatus};
use asiba_modules::docker::{self, Container, DockerSnapshot};
use asiba_modules::services::{self, ServicesSnapshot};

use super::Finding;
use crate::section::DocContext;

const RESTART_LOOP_THRESHOLD: u32 = 5;

pub fn collect(ctx: &DocContext<'_>, out: &mut Vec<Finding>) {
    units(ctx, out);
    containers(ctx, out);
    deploys(ctx, out);
}

fn units(ctx: &DocContext<'_>, out: &mut Vec<Finding>) {
    let Some(snapshot) = ctx.server.data::<ServicesSnapshot>(services::ID) else {
        return;
    };
    for unit in snapshot.failed() {
        out.push(Finding::new(
            Severity::Critical,
            "systemd",
            format!(
                "unit {} failed (result: {}, restarts: {})",
                unit.name, unit.result, unit.restarts
            ),
        ));
    }
}

fn should_be_running(container: &Container) -> bool {
    matches!(
        container.restart_policy.as_str(),
        "always" | "unless-stopped" | "on-failure"
    )
}

fn containers(ctx: &DocContext<'_>, out: &mut Vec<Finding>) {
    let Some(snapshot) = ctx.server.data::<DockerSnapshot>(docker::ID) else {
        return;
    };
    for container in &snapshot.containers {
        if !container.is_running() && (should_be_running(container) || container.exit_code != 0) {
            out.push(Finding::new(
                Severity::Critical,
                "docker",
                format!(
                    "container {} is {} (exit code {}, restart policy {})",
                    container.name, container.state, container.exit_code, container.restart_policy
                ),
            ));
            continue;
        }
        if container.is_unhealthy() {
            out.push(Finding::new(
                Severity::Warning,
                "docker",
                format!("container {} is unhealthy", container.name),
            ));
        }
        if container.restart_count >= RESTART_LOOP_THRESHOLD {
            out.push(Finding::new(
                Severity::Warning,
                "docker",
                format!(
                    "container {} restarted {} times",
                    container.name, container.restart_count
                ),
            ));
        }
    }
}

fn deploys(ctx: &DocContext<'_>, out: &mut Vec<Finding>) {
    let Some(state) = ctx.server.data::<DeployState>(deploy::ID) else {
        return;
    };
    let mut seen = Vec::new();
    for deploy in &state.snapshot.deploys {
        if seen.contains(&deploy.project) {
            continue;
        }
        seen.push(deploy.project.clone());
        if deploy.status != DeployStatus::Failed {
            continue;
        }
        let error = deploy
            .error
            .as_ref()
            .map(|e| e.line.clone())
            .unwrap_or_else(|| deploy.detail.clone());
        out.push(Finding::new(
            Severity::Critical,
            "deploy",
            format!("last deploy of {} failed: {error}", deploy.project),
        ));
    }
}
