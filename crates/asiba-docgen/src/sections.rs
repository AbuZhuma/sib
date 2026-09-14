use std::fmt::Write;

use asiba_core::{ServerState, Severity};
use asiba_modules::docker::{self, DockerSnapshot};
use asiba_modules::ports::{self, PortsSnapshot};
use asiba_modules::processes::{self, ProcessSnapshot};
use asiba_modules::projects::{self, ProjectsSnapshot};
use asiba_modules::services::{self, ServicesSnapshot, UnitOrigin};
use asiba_modules::system::{self, SystemInfo};

const MAX_PROBLEMS: usize = 20;
const MAX_UNASSIGNED: usize = 40;

fn line(out: &mut String, text: impl AsRef<str>) {
    let _ = writeln!(out, "{}", text.as_ref());
}

pub fn description(out: &mut String, server: &ServerState) {
    let d = &server.spec.description;
    line(out, "## Описание\n");
    let rows = [
        ("Проект", d.project.as_str()),
        ("Назначение", d.purpose.as_str()),
        ("Окружение", &format!("{:?}", d.environment).to_lowercase()),
        ("Ответственный", d.owner.as_str()),
        ("Теги", &d.tags.join(", ")),
        ("Ссылки", &d.links.join(", ")),
        (
            "Адрес",
            &format!(
                "{}@{}:{}",
                server.spec.user, server.spec.host, server.spec.port
            ),
        ),
    ];
    for (label, value) in rows.iter().filter(|(_, v)| !v.is_empty()) {
        line(out, format!("- {label}: {value}"));
    }
    if !d.notes.is_empty() {
        line(out, format!("\n{}", d.notes));
    }
    line(out, "");
}

pub fn system(out: &mut String, server: &ServerState) {
    let Some(info) = server.data::<SystemInfo>(system::ID) else {
        return;
    };
    line(out, "## Система\n");
    line(
        out,
        format!("- ОС: {} ({}, {})", info.os_name, info.kernel, info.arch),
    );
    line(
        out,
        format!("- CPU: {} × {}", info.cpu_cores, info.cpu_model),
    );
    line(
        out,
        format!("- RAM: {} МиБ", info.mem_total_bytes / 1024 / 1024),
    );
    if let Some(virt) = &info.virtualization {
        line(out, format!("- Виртуализация: {virt}"));
    }
    line(out, format!("- Uptime: {}", info.uptime_human()));
    line(out, "");
}

pub fn projects(out: &mut String, server: &ServerState) {
    let Some(snapshot) = server.data::<ProjectsSnapshot>(projects::ID) else {
        return;
    };
    line(out, "## Проекты на сервере\n");
    if snapshot.projects.is_empty() {
        line(out, "Проекты не обнаружены.\n");
        return;
    }
    for project in &snapshot.projects {
        line(out, format!("### {} — `{}`\n", project.name, project.path));
        line(out, format!("- Тип: {}", project.kinds_label()));
        if let Some(git) = &project.git {
            let dirty = if git.dirty_files > 0 {
                format!(", изменённых файлов: {}", git.dirty_files)
            } else {
                String::new()
            };
            line(
                out,
                format!("- Git: {} — {}{dirty}", git.branch, git.last_commit),
            );
        }
        if !project.containers.is_empty() {
            line(
                out,
                format!("- Контейнеры: {}", project.containers.join(", ")),
            );
        }
        if !project.units.is_empty() {
            line(out, format!("- Сервисы: {}", project.units.join(", ")));
        }
        if !project.ports.is_empty() {
            let ports: Vec<String> = project.ports.iter().map(u16::to_string).collect();
            line(out, format!("- Порты: {}", ports.join(", ")));
        }
        line(out, "");
    }
}

pub fn processes(out: &mut String, server: &ServerState) {
    let Some(snapshot) = server.data::<ProjectsSnapshot>(projects::ID) else {
        return;
    };
    let total = server
        .data::<ProcessSnapshot>(processes::ID)
        .map(|p| p.processes.len());
    line(out, "## Процессы\n");
    if let Some(total) = total {
        line(out, format!("Всего процессов: {total}.\n"));
    }
    line(out, "| PID | Процесс | Проект |");
    line(out, "|---|---|---|");
    for project in snapshot.active() {
        for (pid, comm) in &project.processes {
            line(out, format!("| {pid} | {comm} | {} |", project.name));
        }
    }
    for process in snapshot.unassigned.iter().take(MAX_UNASSIGNED) {
        line(
            out,
            format!(
                "| {} | {} | не отнесён (`{}`) |",
                process.pid, process.comm, process.cwd
            ),
        );
    }
    line(out, "");
}

pub fn services(out: &mut String, server: &ServerState) {
    let Some(snapshot) = server.data::<ServicesSnapshot>(services::ID) else {
        return;
    };
    line(out, "## Сервисы systemd\n");
    let shown = snapshot
        .units
        .iter()
        .filter(|u| u.origin() == UnitOrigin::Custom || u.is_failed());
    line(out, "| Юнит | Состояние | Перезапусков | Описание |");
    line(out, "|---|---|---|---|");
    for unit in shown {
        line(
            out,
            format!(
                "| {} | {} ({}) | {} | {} |",
                unit.name, unit.active, unit.sub, unit.restarts, unit.description
            ),
        );
    }
    line(out, "");
}

pub fn docker(out: &mut String, server: &ServerState) {
    let Some(snapshot) = server.data::<DockerSnapshot>(docker::ID) else {
        return;
    };
    line(out, format!("## Docker ({})\n", snapshot.version));
    line(out, "| Контейнер | Образ | Состояние | Compose | Порты |");
    line(out, "|---|---|---|---|---|");
    for container in &snapshot.containers {
        let compose = container.compose_project.as_deref().unwrap_or("—");
        line(
            out,
            format!(
                "| {} | {} | {} | {compose} | {} |",
                container.name, container.image, container.status, container.ports
            ),
        );
    }
    line(
        out,
        format!(
            "\nОбразов: {}, томов: {}, сетей: {}.\n",
            snapshot.images.len(),
            snapshot.volumes,
            snapshot.networks.len()
        ),
    );
}

pub fn ports(out: &mut String, server: &ServerState) {
    let Some(snapshot) = server.data::<PortsSnapshot>(ports::ID) else {
        return;
    };
    line(out, "## Порты\n");
    line(out, "| Порт | Протокол | Адрес | Процесс | Снаружи |");
    line(out, "|---|---|---|---|---|");
    for port in &snapshot.ports {
        let reachable = match port.reachable {
            Some(true) => "доступен",
            Some(false) => "закрыт",
            None => "—",
        };
        line(
            out,
            format!(
                "| {} | {} | {} | {} | {reachable} |",
                port.port,
                port.protocol.label(),
                port.address,
                port.process_label()
            ),
        );
    }
    line(out, "");
}

pub fn problems(out: &mut String, server: &ServerState) {
    let problems: Vec<_> = server
        .recent_events
        .iter()
        .rev()
        .filter(|e| e.severity >= Severity::Warning)
        .take(MAX_PROBLEMS)
        .collect();
    if problems.is_empty() {
        return;
    }
    line(out, "## Последние проблемы\n");
    for event in problems {
        let at = event
            .at
            .with_timezone(&chrono::Local)
            .format("%Y-%m-%d %H:%M");
        line(out, format!("- {at} [{}] {}", event.module, event.message));
    }
    line(out, "");
}
