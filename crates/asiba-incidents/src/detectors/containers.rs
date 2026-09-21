use asiba_core::{AppState, IncidentDraft, IncidentKind, ServerState, Severity};
use asiba_modules::docker::{self, Container, DockerSnapshot};

use super::Detector;

pub struct ContainersDetector;

const RESTART_LOOP_THRESHOLD: u32 = 5;
const STALE_STATUS_MARKERS: [&str; 4] = ["days ago", "weeks ago", "months ago", "years ago"];

fn is_stale(container: &Container) -> bool {
    STALE_STATUS_MARKERS
        .iter()
        .any(|marker| container.status.contains(marker))
}

fn should_be_running(container: &Container) -> bool {
    matches!(
        container.restart_policy.as_str(),
        "always" | "unless-stopped" | "on-failure"
    )
}

fn down(container: &Container) -> Option<IncidentDraft> {
    if container.is_running() {
        return None;
    }
    let severity = if should_be_running(container) {
        Severity::Critical
    } else if container.exit_code != 0 && !is_stale(container) {
        Severity::Warning
    } else {
        return None;
    };
    Some(
        IncidentDraft::new(
            IncidentKind::ContainerDown,
            severity,
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

#[cfg(test)]
mod tests {
    use super::*;

    fn container(state: &str, status: &str, policy: &str, exit_code: i32) -> Container {
        Container {
            id: "abc".into(),
            name: "web".into(),
            image: "nginx".into(),
            state: state.into(),
            status: status.into(),
            ports: String::new(),
            restart_count: 0,
            health: None,
            restart_policy: policy.into(),
            exit_code,
            started_at: String::new(),
            compose_project: None,
            compose_service: None,
            compose_dir: None,
            stats: None,
        }
    }

    #[test]
    fn stopped_container_with_restart_policy_is_critical() {
        let draft = down(&container(
            "exited",
            "Exited (0) 2 minutes ago",
            "always",
            0,
        ));
        assert_eq!(draft.map(|d| d.severity), Some(Severity::Critical));
    }

    #[test]
    fn failed_one_off_container_is_warning_until_it_gets_old() {
        let fresh = down(&container("exited", "Exited (1) 5 minutes ago", "no", 1));
        assert_eq!(fresh.map(|d| d.severity), Some(Severity::Warning));
        let old = down(&container("exited", "Exited (1) 6 days ago", "no", 1));
        assert!(old.is_none());
        let clean = down(&container("exited", "Exited (0) 5 minutes ago", "no", 0));
        assert!(clean.is_none());
    }
}
