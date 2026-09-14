use asiba_core::{ModuleId, ServerState};
use asiba_modules::projects::{self, Project, ProjectsSnapshot};
use egui::{Frame, Grid, Margin, RichText, Stroke, Ui};

use super::{ModuleView, Tab, ViewShared};
use crate::components::{Table, badge};
use crate::text;
use crate::theme::{GAP, Palette};

const MAX_UNASSIGNED: usize = 60;

pub struct ProjectsView;

impl ModuleView for ProjectsView {
    fn id(&self) -> ModuleId {
        projects::ID
    }

    fn title(&self) -> &'static str {
        text::MODULE_PROJECTS
    }

    fn tab(&self) -> Tab {
        Tab::Projects
    }

    fn summary(&self, ui: &mut Ui, server: &ServerState) {
        let p = Palette::current(ui.ctx());
        let Some(snapshot) = server.data::<ProjectsSnapshot>(projects::ID) else {
            return;
        };
        if snapshot.projects.is_empty() {
            ui.label(RichText::new(text::PROJ_NONE).color(p.text_muted));
            return;
        }
        for project in &snapshot.projects {
            ui.horizontal(|ui| {
                ui.label(RichText::new(&project.name).strong());
                badge(ui, &project.kinds_label(), p.text_secondary);
                if let Some(git) = &project.git {
                    ui.monospace(RichText::new(&git.branch).color(p.text_secondary));
                }
                ui.label(RichText::new(&project.path).small().color(p.text_muted));
            });
        }
    }

    fn page(
        &self,
        ui: &mut Ui,
        server: &ServerState,
        _shared: &ViewShared,
    ) -> Option<super::ViewAction> {
        let snapshot = server.data::<ProjectsSnapshot>(projects::ID)?;
        for project in &snapshot.projects {
            card(ui, project);
            ui.add_space(GAP);
        }
        if !snapshot.unassigned.is_empty() {
            unassigned(ui, snapshot);
        }
        None
    }
}

fn card(ui: &mut Ui, project: &Project) {
    let p = Palette::current(ui.ctx());
    Frame::new()
        .fill(p.bg_raised)
        .stroke(Stroke::new(1.0, p.border))
        .inner_margin(Margin::same(GAP as i8))
        .show(ui, |ui| {
            ui.set_min_width(ui.available_width());
            ui.horizontal(|ui| {
                ui.label(RichText::new(&project.name).heading());
                badge(ui, &project.kinds_label(), p.text_secondary);
                ui.monospace(RichText::new(&project.path).color(p.text_muted));
            });
            Grid::new(("project", &project.path))
                .num_columns(2)
                .spacing([16.0, 2.0])
                .show(ui, |ui| {
                    let mut row = |label: &str, value: String| {
                        if value.is_empty() {
                            return;
                        }
                        ui.label(RichText::new(label).color(p.text_secondary));
                        ui.monospace(value);
                        ui.end_row();
                    };
                    if let Some(git) = &project.git {
                        let dirty = if git.dirty_files > 0 {
                            format!("  ({} {})", git.dirty_files, text::PROJ_DIRTY)
                        } else {
                            String::new()
                        };
                        row(
                            text::PROJ_GIT,
                            format!("{} — {}{dirty}", git.branch, git.last_commit),
                        );
                    }
                    row(text::PROJ_CONTAINERS, project.containers.join(", "));
                    row(text::PROJ_UNITS, project.units.join(", "));
                    let processes: Vec<String> = project
                        .processes
                        .iter()
                        .map(|(pid, comm)| format!("{comm}[{pid}]"))
                        .collect();
                    row(text::PROJ_PROCESSES, processes.join(", "));
                    let ports: Vec<String> = project.ports.iter().map(u16::to_string).collect();
                    row(text::PROJ_PORTS, ports.join(", "));
                });
        });
}

fn unassigned(ui: &mut Ui, snapshot: &ProjectsSnapshot) {
    let p = Palette::current(ui.ctx());
    ui.label(
        RichText::new(text::PROJ_UNASSIGNED.to_uppercase())
            .small()
            .color(p.text_secondary),
    );
    let columns = ["PID", text::PROC_COMMAND, text::PROJ_PATH];
    Table::new("projects-unassigned", &columns).show(ui, |ui| {
        for process in snapshot.unassigned.iter().take(MAX_UNASSIGNED) {
            ui.monospace(process.pid.to_string());
            ui.monospace(&process.comm);
            ui.monospace(&process.cwd);
            ui.end_row();
        }
    });
}
