use asiba_core::{ModuleId, ServerState};
use asiba_modules::projects::{self, Project, ProjectsSnapshot};
use egui::{Frame, Grid, Margin, RichText, Stroke, Ui};

use super::{ModuleView, Tab, ViewShared};
use crate::components::{Table, badge};
use crate::text;
use crate::theme::{GAP, GAP_SMALL, Palette};

const MAX_UNASSIGNED: usize = 60;
const SUMMARY_PROCESSES: usize = 6;

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
                if project.is_active() {
                    badge(ui, text::PROJ_ACTIVE, p.ok);
                }
                if let Some(git) = &project.git {
                    ui.monospace(RichText::new(&git.branch).color(p.text_secondary));
                }
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
            ui.horizontal_wrapped(|ui| {
                ui.label(RichText::new(&project.name).heading());
                for kind in &project.kinds {
                    badge(ui, kind.label(), p.text_secondary);
                }
                if project.is_active() {
                    badge(ui, text::PROJ_ACTIVE, p.ok);
                } else {
                    badge(ui, text::PROJ_IDLE, p.text_muted);
                }
            });
            ui.monospace(RichText::new(&project.path).color(p.text_muted));
            ui.add_space(GAP_SMALL);
            Grid::new(("project", &project.path))
                .num_columns(2)
                .spacing([16.0, 4.0])
                .show(ui, |ui| details(ui, project, &p));
        });
}

fn details(ui: &mut Ui, project: &Project, p: &Palette) {
    if let Some(git) = &project.git {
        git_rows(ui, git, p);
    }
    badge_row(ui, text::PROJ_CONTAINERS, &project.containers, p.info, p);
    badge_row(ui, text::PROJ_UNITS, &project.units, p.text_secondary, p);
    let ports: Vec<String> = project.ports.iter().map(u16::to_string).collect();
    badge_row(ui, text::PROJ_PORTS, &ports, p.text_secondary, p);
    if !project.processes.is_empty() {
        ui.label(RichText::new(text::PROJ_PROCESSES).color(p.text_secondary));
        ui.label(process_summary(&project.processes));
        ui.end_row();
    }
}

fn git_rows(ui: &mut Ui, git: &asiba_modules::projects::GitInfo, p: &Palette) {
    ui.label(RichText::new(text::PROJ_BRANCH).color(p.text_secondary));
    ui.horizontal(|ui| {
        ui.monospace(&git.branch);
        if git.dirty_files > 0 {
            badge(
                ui,
                &format!("{} {}", git.dirty_files, text::PROJ_DIRTY),
                p.warning,
            );
        }
    });
    ui.end_row();
    let (hash, date, subject) = split_commit(&git.last_commit);
    ui.label(RichText::new(text::PROJ_COMMIT).color(p.text_secondary));
    ui.horizontal_wrapped(|ui| {
        ui.monospace(hash);
        ui.monospace(RichText::new(date).color(p.text_secondary));
        ui.label(subject);
    });
    ui.end_row();
}

fn badge_row(ui: &mut Ui, label: &str, items: &[String], color: egui::Color32, p: &Palette) {
    if items.is_empty() {
        return;
    }
    ui.label(RichText::new(label).color(p.text_secondary));
    ui.horizontal_wrapped(|ui| {
        for item in items {
            badge(ui, item, color);
        }
    });
    ui.end_row();
}

fn split_commit(raw: &str) -> (&str, String, &str) {
    let mut tokens = raw.splitn(4, ' ');
    let hash = tokens.next().unwrap_or_default();
    let date = tokens.next().unwrap_or_default();
    let time = tokens.next().unwrap_or_default();
    let rest = tokens.next().unwrap_or_default();
    let subject = rest.split_once(' ').map(|(_, s)| s).unwrap_or(rest);
    let clock: String = time.chars().take(5).collect();
    (hash, format!("{date} {clock}"), subject)
}

fn process_summary(processes: &[(u32, String)]) -> String {
    let mut counts: Vec<(&str, usize)> = Vec::new();
    for (_, comm) in processes {
        match counts.iter_mut().find(|(name, _)| *name == comm.as_str()) {
            Some((_, count)) => *count += 1,
            None => counts.push((comm.as_str(), 1)),
        }
    }
    counts.sort_by_key(|(_, count)| std::cmp::Reverse(*count));
    let shown: Vec<String> = counts
        .iter()
        .take(SUMMARY_PROCESSES)
        .map(|(name, count)| {
            if *count > 1 {
                format!("{name} ×{count}")
            } else {
                (*name).to_owned()
            }
        })
        .collect();
    let more = counts.len().saturating_sub(SUMMARY_PROCESSES);
    let suffix = if more > 0 {
        format!(" +{more}")
    } else {
        String::new()
    };
    format!("{} - {}{suffix}", processes.len(), shown.join(", "))
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn split_commit_separates_hash_date_and_subject() {
        let (hash, date, subject) =
            split_commit("952e432 2026-09-14 19:49:04 +0600 Split worker management");
        assert_eq!(hash, "952e432");
        assert_eq!(date, "2026-09-14 19:49");
        assert_eq!(subject, "Split worker management");
    }

    #[test]
    fn process_summary_groups_by_name() {
        let processes = vec![
            (1, "bash".to_owned()),
            (2, "node".to_owned()),
            (3, "node".to_owned()),
        ];
        assert_eq!(process_summary(&processes), "3 - node ×2, bash");
    }
}
