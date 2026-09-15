use asiba_modules::projects::{self, Project, ProjectsSnapshot};

use crate::section::{DocContext, Section, SectionId};
use crate::write::{blank, bullet, field, heading, line, subheading};

pub struct ProjectsSection;

fn snapshot<'a>(ctx: &'a DocContext<'_>) -> Option<&'a ProjectsSnapshot> {
    ctx.server.data::<ProjectsSnapshot>(projects::ID)
}

fn git_label(project: &Project) -> Option<String> {
    let git = project.git.as_ref()?;
    let dirty = if git.dirty_files > 0 {
        format!(", dirty files: {}", git.dirty_files)
    } else {
        String::new()
    };
    Some(format!("{} - {}{dirty}", git.branch, git.last_commit))
}

fn ports_label(project: &Project) -> String {
    let ports: Vec<String> = project.ports.iter().map(u16::to_string).collect();
    ports.join(", ")
}

fn rows(project: &Project) -> Vec<(&'static str, &'static str, String)> {
    let mut rows = vec![
        ("Путь", "path", project.path.clone()),
        ("Тип", "kind", project.kinds_label()),
    ];
    if let Some(git) = git_label(project) {
        rows.push(("Git", "git", git));
    }
    rows.push(("Контейнеры", "containers", project.containers.join(", ")));
    rows.push(("Сервисы", "units", project.units.join(", ")));
    rows.push(("Порты", "ports", ports_label(project)));
    rows.push((
        "Процессов",
        "process_count",
        project.processes.len().to_string(),
    ));
    rows.into_iter().filter(|(_, _, v)| !v.is_empty()).collect()
}

impl Section for ProjectsSection {
    fn id(&self) -> SectionId {
        SectionId::Projects
    }

    fn is_available(&self, ctx: &DocContext<'_>) -> bool {
        snapshot(ctx).is_some()
    }

    fn human(&self, out: &mut String, ctx: &DocContext<'_>) {
        let Some(snapshot) = snapshot(ctx) else {
            return;
        };
        heading(out, "Проекты на сервере");
        if snapshot.projects.is_empty() {
            line(out, "Проекты не обнаружены.\n");
            return;
        }
        for project in &snapshot.projects {
            subheading(out, &project.name);
            for (label, _, value) in rows(project) {
                bullet(out, label, value);
            }
            blank(out);
        }
    }

    fn llm(&self, out: &mut String, ctx: &DocContext<'_>) {
        let Some(snapshot) = snapshot(ctx) else {
            return;
        };
        heading(out, "Projects");
        if snapshot.projects.is_empty() {
            line(out, "none detected\n");
            return;
        }
        for project in &snapshot.projects {
            field(out, "project", &project.name);
            for (_, key, value) in rows(project) {
                field(out, &format!("  {key}"), value);
            }
        }
        blank(out);
    }
}
