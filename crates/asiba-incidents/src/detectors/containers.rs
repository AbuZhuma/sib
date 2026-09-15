use asiba_core::{AppState, IncidentDraft, IncidentKind, ServerState, Severity};
use asiba_modules::docker::{self, Container, DockerSnapshot};

use super::Detector;

pub struct ContainersDetector;

const RESTART_LOOP_THRESHOLD: u32 = 5;

fn should_be_running(container: &Container) -> bool {
    matches!(
        container.restart_policy.as_str(),
        "always" | "unless-stopped" | "on-failure"
    )
}

fn down(container: &Container) -> Option<IncidentDraft> {
    if container.is_running() || !(should_be_running(container) || container.exit_code != 0) {
        return None;
    }
    Some(
        IncidentDraft::new(
            IncidentKind::ContainerDown,
            Severity::Critical,
            container.name.clone(),
            format!(
                "container {} is {} (exit code {}, restart policy {})",
                container.name, container.state, container.exit_code, container.restart_policy
            ),
        )
        .evidence([
            format!("image: {}", container.image),
            format!("status: {}", container.status),
            format!(
                "compose: {}",
                container.compose_project.as_deref().unwrap_or("-")
            ),
        ]),
    )
}

fn unhealthy(container: &Container) -> Option<IncidentDraft> {
    if !container.is_running() {
        return None;
    }
    if container.is_unhealthy() {
        return Some(IncidentDraft::new(
            IncidentKind::ContainerDown,
            Severity::Warning,
            container.name.clone(),
            format!("container {} is unhealthy", container.name),
        ));
    }
    if container.restart_count >= RESTART_LOOP_THRESHOLD {
        return Some(IncidentDraft::new(
            IncidentKind::ContainerDown,
            Severity::Warning,
            container.name.clone(),
            format!(
                "container {} restarted {} times",
                container.name, container.restart_count
            ),
        ));
    }
    None
}

impl Detector for ContainersDetector {
    fn detect(&self, server: &ServerState, _state: &AppState) -> Vec<IncidentDraft> {
        let Some(snapshot) = server.data::<DockerSnapshot>(docker::ID) else {
            return Vec::new();
        };
        snapshot
            .containers
            .iter()
            .filter_map(|c| down(c).or_else(|| unhealthy(c)))
            .collect()
    }
}
