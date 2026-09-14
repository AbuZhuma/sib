use asiba_core::ModuleError;

use super::model::{Container, ContainerStats, DockerSnapshot, Image};
use crate::common::sections::Sections;
use crate::common::size;

pub fn docker_snapshot(raw: &str) -> Result<DockerSnapshot, ModuleError> {
    let sections = Sections::parse(raw);
    let version = sections.get_or_empty("version").trim().to_owned();
    if version.is_empty() {
        return Err(ModuleError::Parse("docker не отвечает".to_owned()));
    }
    let mut containers: Vec<Container> = sections
        .get_or_empty("ps")
        .lines()
        .filter_map(ps_line)
        .collect();
    apply_inspect(&mut containers, sections.get_or_empty("inspect"));
    apply_stats(&mut containers, sections.get_or_empty("stats"));
    Ok(DockerSnapshot {
        version,
        containers,
        images: sections
            .get_or_empty("images")
            .lines()
            .filter_map(image_line)
            .collect(),
        volumes: sections.get_or_empty("volumes").trim().parse().unwrap_or(0),
        networks: sections
            .get_or_empty("networks")
            .lines()
            .map(str::to_owned)
            .collect(),
    })
}

fn ps_line(line: &str) -> Option<Container> {
    let fields: Vec<&str> = line.split('\t').collect();
    if fields.len() < 5 {
        return None;
    }
    Some(Container {
        id: fields[0].to_owned(),
        name: fields[1].to_owned(),
        image: fields[2].to_owned(),
        state: fields[3].to_owned(),
        status: fields[4].to_owned(),
        ports: fields.get(5).copied().unwrap_or_default().to_owned(),
        restart_count: 0,
        health: None,
        restart_policy: String::new(),
        exit_code: 0,
        started_at: String::new(),
        compose_project: None,
        compose_service: None,
        compose_dir: None,
        stats: None,
    })
}

fn apply_inspect(containers: &mut [Container], raw: &str) {
    for line in raw.lines() {
        let fields: Vec<&str> = line.split('\t').collect();
        if fields.len() < 10 {
            continue;
        }
        let Some(container) = containers.iter_mut().find(|c| fields[0].starts_with(&c.id)) else {
            continue;
        };
        container.restart_count = fields[2].parse().unwrap_or(0);
        container.health = optional(fields[3]);
        container.restart_policy = fields[4].to_owned();
        container.exit_code = fields[5].parse().unwrap_or(0);
        container.started_at = fields[6].to_owned();
        container.compose_project = optional(fields[7]);
        container.compose_service = optional(fields[8]);
        container.compose_dir = optional(fields[9]);
    }
}

fn apply_stats(containers: &mut [Container], raw: &str) {
    for line in raw.lines() {
        let fields: Vec<&str> = line.split('\t').collect();
        if fields.len() < 7 {
            continue;
        }
        let Some(container) = containers
            .iter_mut()
            .find(|c| fields[0].starts_with(&c.id) || c.id.starts_with(fields[0]))
        else {
            continue;
        };
        let (mem_usage, mem_limit) = size::parse_pair(fields[3]).unwrap_or((0, 0));
        let (net_rx, net_tx) = size::parse_pair(fields[4]).unwrap_or((0, 0));
        let (block_read, block_write) = size::parse_pair(fields[5]).unwrap_or((0, 0));
        container.stats = Some(ContainerStats {
            cpu_pct: size::parse_percent(fields[2]).unwrap_or(0.0),
            mem_usage,
            mem_limit,
            net_rx,
            net_tx,
            block_read,
            block_write,
            pids: fields[6].trim().parse().unwrap_or(0),
        });
    }
}

fn image_line(line: &str) -> Option<Image> {
    let fields: Vec<&str> = line.split('\t').collect();
    if fields.len() < 5 {
        return None;
    }
    Some(Image {
        id: fields[0].to_owned(),
        repository: fields[1].to_owned(),
        tag: fields[2].to_owned(),
        size_bytes: size::parse_bytes(fields[3]).unwrap_or(0),
        created: fields[4].to_owned(),
    })
}

fn optional(value: &str) -> Option<String> {
    let trimmed = value.trim().trim_start_matches('/');
    (!trimmed.is_empty()).then(|| trimmed.to_owned())
}

#[cfg(test)]
mod tests {
    use super::*;

    const DOCKER: &str = include_str!("../../fixtures/docker/docker.txt");
    const PODMAN: &str = include_str!("../../fixtures/docker/podman.txt");

    #[test]
    fn docker_fixture_joins_ps_inspect_and_stats() {
        let snapshot = docker_snapshot(DOCKER).expect("parse");
        assert_eq!(snapshot.version, "27.3.1");
        assert_eq!(snapshot.containers.len(), 2);
        let web = snapshot
            .containers
            .iter()
            .find(|c| c.name == "shop-web-1")
            .expect("web");
        assert_eq!(web.compose_project.as_deref(), Some("shop"));
        assert_eq!(web.health.as_deref(), Some("healthy"));
        assert_eq!(web.restart_count, 2);
        let stats = web.stats.as_ref().expect("stats");
        assert_eq!(stats.cpu_pct, 1.25);
        assert_eq!(stats.mem_usage, 52_430_000);
        let db = snapshot
            .containers
            .iter()
            .find(|c| c.name == "shop-db-1")
            .expect("db");
        assert!(!db.is_running());
        assert_eq!(db.exit_code, 137);
        assert_eq!(snapshot.images.len(), 2);
        assert!(snapshot.images[1].is_dangling());
        assert_eq!(snapshot.volumes, 3);
    }

    #[test]
    fn podman_fixture_without_compose_labels_parses() {
        let snapshot = docker_snapshot(PODMAN).expect("parse");
        assert_eq!(snapshot.containers.len(), 1);
        assert_eq!(snapshot.containers[0].compose_project, None);
        assert_eq!(snapshot.compose_projects(), Vec::<String>::new());
    }

    #[test]
    fn missing_version_is_parse_error() {
        assert!(matches!(
            docker_snapshot("###version\n\n###ps\n"),
            Err(ModuleError::Parse(_))
        ));
    }
}
