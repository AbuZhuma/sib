use asiba_core::{ModuleId, QueryRequest, ServerState};
use asiba_modules::docker::{self, Container, DockerSnapshot};
use egui::{RichText, Ui};

use super::{ModuleView, Tab, ViewAction, ViewShared, action_button};
use crate::components::{Table, badge};
use crate::format;
use crate::text;
use crate::theme::{GAP, Palette};

const SUMMARY_CONTAINERS: usize = 8;

pub struct DockerView;

impl ModuleView for DockerView {
    fn id(&self) -> ModuleId {
        docker::ID
    }

    fn title(&self) -> &'static str {
        text::MODULE_DOCKER
    }

    fn tab(&self) -> Tab {
        Tab::Docker
    }

    fn summary(&self, ui: &mut Ui, server: &ServerState) {
        let p = Palette::current(ui.ctx());
        let Some(snapshot) = server.data::<DockerSnapshot>(docker::ID) else {
            return;
        };
        ui.horizontal_wrapped(|ui| {
            ui.monospace(format!(
                "{} {}",
                snapshot.running_count(),
                text::DOCKER_RUNNING
            ));
            let stopped = snapshot.stopped_count();
            let color = if stopped > 0 { p.warning } else { p.text };
            ui.label(
                RichText::new(format!("{stopped} {}", text::DOCKER_STOPPED))
                    .monospace()
                    .color(color),
            );
            ui.monospace(format!("{} {}", snapshot.images.len(), text::DOCKER_IMAGES));
            ui.monospace(format!("{} {}", snapshot.volumes, text::DOCKER_VOLUMES));
        });
        for container in snapshot
            .containers
            .iter()
            .filter(|c| !c.is_running() || c.is_unhealthy())
            .take(5)
        {
            ui.label(
                RichText::new(format!("{} — {}", container.name, container.status))
                    .color(p.warning),
            );
        }
        running_table(ui, snapshot, &p);
    }

    fn page(&self, ui: &mut Ui, server: &ServerState, _shared: &ViewShared) -> Option<ViewAction> {
        let snapshot = server.data::<DockerSnapshot>(docker::ID)?;
        let mut action = None;
        let projects = snapshot.compose_projects();
        for project in &projects {
            let members: Vec<&Container> = snapshot
                .containers
                .iter()
                .filter(|c| c.compose_project.as_deref() == Some(project))
                .collect();
            section_title(ui, &format!("{} {project}", text::DOCKER_COMPOSE));
            if let Some(next) = containers_table(ui, &format!("docker-{project}"), &members) {
                action = Some(next);
            }
            ui.add_space(GAP);
        }
        let loose: Vec<&Container> = snapshot
            .containers
            .iter()
            .filter(|c| c.compose_project.is_none())
            .collect();
        if !loose.is_empty() {
            if !projects.is_empty() {
                section_title(ui, text::DOCKER_CONTAINER);
            }
            if let Some(next) = containers_table(ui, "docker-loose", &loose) {
                action = Some(next);
            }
            ui.add_space(GAP);
        }
        images_table(ui, snapshot);
        action
    }
}

fn running_table(ui: &mut Ui, snapshot: &DockerSnapshot, p: &Palette) {
    let running: Vec<&Container> = snapshot
        .containers
        .iter()
        .filter(|c| c.is_running())
        .take(SUMMARY_CONTAINERS)
        .collect();
    if running.is_empty() {
        return;
    }
    ui.add_space(GAP);
    let columns = [text::DOCKER_CONTAINER, "CPU", "RAM", text::COL_STATUS];
    Table::new("docker-summary", &columns).show(ui, |ui| {
        for container in running {
            ui.monospace(&container.name);
            let (cpu, mem) = container
                .stats
                .as_ref()
                .map(|s| (format!("{:.1}%", s.cpu_pct), format::bytes(s.mem_usage)))
                .unwrap_or_else(|| ("—".to_owned(), "—".to_owned()));
            ui.monospace(cpu);
            ui.monospace(mem);
            ui.label(RichText::new(&container.status).color(p.text_secondary));
            ui.end_row();
        }
    });
}

fn section_title(ui: &mut Ui, title: &str) {
    let p = Palette::current(ui.ctx());
    ui.label(
        RichText::new(title.to_uppercase())
            .small()
            .color(p.text_secondary),
    );
}

fn containers_table(ui: &mut Ui, id: &str, containers: &[&Container]) -> Option<ViewAction> {
    let p = Palette::current(ui.ctx());
    let mut action = None;
    let columns = [
        text::DOCKER_CONTAINER,
        text::COL_STATUS,
        "CPU",
        "RAM",
        "NET RX/TX",
        text::DOCKER_IMAGE,
        text::DOCKER_PORTS,
        "",
    ];
    Table::new(id, &columns).show(ui, |ui| {
        for container in containers {
            ui.monospace(&container.name);
            status_cell(ui, container, &p);
            match &container.stats {
                Some(stats) => {
                    ui.monospace(format!("{:.1}%", stats.cpu_pct));
                    ui.monospace(format!(
                        "{} / {}",
                        format::bytes(stats.mem_usage),
                        format::bytes(stats.mem_limit)
                    ));
                    ui.monospace(format!(
                        "{} / {}",
                        format::bytes(stats.net_rx),
                        format::bytes(stats.net_tx)
                    ));
                }
                None => {
                    for _ in 0..3 {
                        ui.monospace("—");
                    }
                }
            }
            ui.label(RichText::new(&container.image).color(p.text_secondary));
            ui.monospace(&container.ports);
            ui.horizontal(|ui| {
                if ui.small_button(text::DOCKER_LOGS).clicked() {
                    action = Some(ViewAction::Query(QueryRequest::new(
                        docker::QUERY_LOGS,
                        &container.name,
                    )));
                }
                if let Some(next) = container_buttons(ui, container) {
                    action = Some(next);
                }
            });
            ui.end_row();
        }
    });
    action
}

fn status_cell(ui: &mut Ui, container: &Container, p: &Palette) {
    let color = if container.is_unhealthy() {
        p.critical
    } else if container.is_running() {
        p.ok
    } else if container.exit_code == 0 {
        p.text_muted
    } else {
        p.critical
    };
    ui.horizontal(|ui| {
        badge(ui, &container.state, color);
        if let Some(health) = &container.health {
            ui.label(RichText::new(health).small().color(color));
        }
        if container.restart_count > 0 {
            ui.label(
                RichText::new(format!("↻{}", container.restart_count))
                    .small()
                    .color(p.warning),
            );
        }
    });
}

fn images_table(ui: &mut Ui, snapshot: &DockerSnapshot) {
    let p = Palette::current(ui.ctx());
    section_title(
        ui,
        &format!(
            "{} ({})",
            text::DOCKER_IMAGES,
            format::bytes(snapshot.images_size())
        ),
    );
    let columns = [text::DOCKER_IMAGE, "TAG", "ID", text::DISK_TOTAL, ""];
    Table::new("docker-images", &columns).show(ui, |ui| {
        for image in &snapshot.images {
            ui.monospace(&image.repository);
            ui.monospace(&image.tag);
            ui.monospace(&image.id);
            ui.monospace(format::bytes(image.size_bytes));
            if image.is_dangling() {
                badge(ui, text::DOCKER_DANGLING, p.warning);
            } else {
                ui.label(RichText::new(&image.created).color(p.text_muted));
            }
            ui.end_row();
        }
    });
}

fn container_buttons(ui: &mut Ui, container: &Container) -> Option<ViewAction> {
    let name = &container.name;
    if container.is_running() {
        action_button(ui, text::ACT_RESTART, docker::SPEC_RESTART, name)
            .or_else(|| action_button(ui, text::ACT_STOP, docker::SPEC_STOP, name))
    } else {
        action_button(ui, text::ACT_START, docker::SPEC_START, name)
    }
}
