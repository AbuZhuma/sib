use std::collections::BTreeMap;

use asiba_core::ModuleError;

use super::model::{GitInfo, Project, ProjectKind, ProjectsSnapshot, UnassignedProcess};
use crate::common::sections::Sections;

struct RawContainer {
    name: String,
    compose_project: String,
    working_dir: String,
}

struct RawProcess {
    pid: u32,
    cwd: String,
    comm: String,
}

pub fn projects_snapshot(raw: &str) -> Result<ProjectsSnapshot, ModuleError> {
    let sections = Sections::parse(raw);
    let mut projects = from_markers(sections.get_or_empty("markers"));
    apply_git(&mut projects, sections.get_or_empty("git"));
    apply_units(&mut projects, sections.get_or_empty("units"));
    apply_containers(&mut projects, sections.get_or_empty("containers"));
    let processes: Vec<RawProcess> = sections
        .get_or_empty("cwd")
        .lines()
        .filter_map(process_line)
        .collect();
    let unassigned = apply_processes(&mut projects, &processes);
    apply_ports(&mut projects, sections.get_or_empty("listen"));
    Ok(ProjectsSnapshot {
        projects,
        unassigned,
    })
}

fn from_markers(raw: &str) -> Vec<Project> {
    let mut by_path: BTreeMap<String, Vec<ProjectKind>> = BTreeMap::new();
    for line in raw.lines().map(str::trim).filter(|l| !l.is_empty()) {
        let Some((dir, file)) = line.rsplit_once('/') else {
            continue;
        };
        if let Some(kind) = ProjectKind::from_marker(file) {
            by_path.entry(dir.to_owned()).or_default().push(kind);
        }
    }
    by_path
        .into_iter()
        .map(|(path, mut kinds)| {
            kinds.sort();
            kinds.dedup();
            let name = path.rsplit('/').next().unwrap_or(&path).to_owned();
            Project {
                name,
                path,
                kinds,
                git: None,
                units: Vec::new(),
                containers: Vec::new(),
                processes: Vec::new(),
                ports: Vec::new(),
            }
        })
        .collect()
}

fn apply_git(projects: &mut [Project], raw: &str) {
    for line in raw.lines() {
        let fields: Vec<&str> = line.split('\t').collect();
        if fields.len() < 4 {
            continue;
        }
        if let Some(project) = projects.iter_mut().find(|p| p.path == fields[0]) {
            project.git = Some(GitInfo {
                branch: fields[1].to_owned(),
                last_commit: fields[2].to_owned(),
                dirty_files: fields[3].trim().parse().unwrap_or(0),
            });
        }
    }
}

fn apply_units(projects: &mut [Project], raw: &str) {
    for block in raw.split("\n\n") {
        let fields: BTreeMap<&str, &str> =
            block.lines().filter_map(|l| l.split_once('=')).collect();
        let (Some(id), Some(dir)) = (fields.get("Id"), fields.get("WorkingDirectory")) else {
            continue;
        };
        if let Some(project) = owner_of(projects, dir) {
            project.units.push((*id).to_owned());
        }
    }
}

fn apply_containers(projects: &mut [Project], raw: &str) {
    let containers: Vec<RawContainer> = raw.lines().filter_map(container_line).collect();
    for container in containers {
        let by_dir = owner_of(projects, &container.working_dir);
        let project = match by_dir {
            Some(project) => Some(project),
            None => projects.iter_mut().find(|p| {
                !container.compose_project.is_empty() && p.name == container.compose_project
            }),
        };
        if let Some(project) = project {
            project.containers.push(container.name);
        }
    }
}

fn container_line(line: &str) -> Option<RawContainer> {
    let fields: Vec<&str> = line.split('\t').collect();
    if fields.len() < 3 {
        return None;
    }
    Some(RawContainer {
        name: fields[0].trim_start_matches('/').to_owned(),
        compose_project: fields[1].to_owned(),
        working_dir: fields[2].to_owned(),
    })
}

fn process_line(line: &str) -> Option<RawProcess> {
    let fields: Vec<&str> = line.split('\t').collect();
    if fields.len() < 3 {
        return None;
    }
    Some(RawProcess {
        pid: fields[0].parse().ok()?,
        cwd: fields[1].to_owned(),
        comm: fields[2].trim().to_owned(),
    })
}

fn apply_processes(projects: &mut [Project], processes: &[RawProcess]) -> Vec<UnassignedProcess> {
    let mut unassigned = Vec::new();
    for process in processes {
        if process.cwd.is_empty() || process.cwd == "/" {
            continue;
        }
        match owner_of(projects, &process.cwd) {
            Some(project) => project.processes.push((process.pid, process.comm.clone())),
            None => unassigned.push(UnassignedProcess {
                pid: process.pid,
                comm: process.comm.clone(),
                cwd: process.cwd.clone(),
            }),
        }
    }
    unassigned
}

fn apply_ports(projects: &mut [Project], raw: &str) {
    for line in raw.lines() {
        let fields: Vec<&str> = line.split_whitespace().collect();
        let (Some(local), Some(users)) = (fields.get(3), fields.get(5)) else {
            continue;
        };
        let Some(port) = local
            .rsplit_once(':')
            .and_then(|(_, p)| p.parse::<u16>().ok())
        else {
            continue;
        };
        let pids: Vec<u32> = users
            .split("pid=")
            .skip(1)
            .filter_map(|r| r.split([',', ')']).next()?.parse().ok())
            .collect();
        for project in projects.iter_mut() {
            if project.processes.iter().any(|(pid, _)| pids.contains(pid))
                && !project.ports.contains(&port)
            {
                project.ports.push(port);
            }
        }
    }
}

fn owner_of<'a>(projects: &'a mut [Project], path: &str) -> Option<&'a mut Project> {
    if path.is_empty() {
        return None;
    }
    projects
        .iter_mut()
        .filter(|p| path == p.path || path.starts_with(&format!("{}/", p.path)))
        .max_by_key(|p| p.path.len())
}

#[cfg(test)]
mod tests {
    use super::*;

    const SERVER: &str = include_str!("../../fixtures/projects/server.txt");

    #[test]
    fn fixture_groups_markers_into_projects() {
        let snapshot = projects_snapshot(SERVER).expect("parse");
        let names: Vec<&str> = snapshot.projects.iter().map(|p| p.name.as_str()).collect();
        assert_eq!(names, vec!["shop", "tools"]);
        let shop = &snapshot.projects[0];
        assert_eq!(
            shop.kinds,
            vec![ProjectKind::Git, ProjectKind::Compose, ProjectKind::Node]
        );
        assert_eq!(shop.git.as_ref().map(|g| g.branch.as_str()), Some("main"));
        assert_eq!(shop.units, vec!["shop-worker.service"]);
        assert_eq!(shop.containers, vec!["shop-web-1", "shop-db-1"]);
        assert_eq!(shop.processes, vec![(2001, "node".to_owned())]);
        assert_eq!(shop.ports, vec![3000]);
    }

    #[test]
    fn processes_outside_projects_are_unassigned() {
        let snapshot = projects_snapshot(SERVER).expect("parse");
        assert_eq!(snapshot.unassigned.len(), 1);
        assert_eq!(snapshot.unassigned[0].comm, "python3");
    }
}
