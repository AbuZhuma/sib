use sib_modules::docker::{self, Container, DockerSnapshot};

use crate::section::{DocContext, Section, SectionId};
use crate::write::{NONE, blank, bytes, field, heading, list, table};

pub struct DockerSection;

const MAX_CONTAINERS: usize = 40;

fn snapshot<'a>(ctx: &'a DocContext<'_>) -> Option<&'a DockerSnapshot> {
    ctx.server.data::<DockerSnapshot>(docker::ID)
}

fn row(container: &Container) -> Vec<String> {
    vec![
        container.name.clone(),
        container.image.clone(),
        container.state.clone(),
        container.status.clone(),
        container.health.clone().unwrap_or_else(|| NONE.to_owned()),
        container.restart_count.to_string(),
        container.exit_code.to_string(),
        container
            .compose_project
            .clone()
            .unwrap_or_else(|| NONE.to_owned()),
        container.ports.clone(),
    ]
}

fn rows(snapshot: &DockerSnapshot) -> Vec<Vec<String>> {
    let mut containers: Vec<&Container> = snapshot.containers.iter().collect();
    containers.sort_by_key(|c| c.is_running());
    containers
        .iter()
        .take(MAX_CONTAINERS)
        .map(|c| row(c))
        .collect()
}

const HEADERS_HUMAN: [&str; 9] = [
    "Контейнер",
    "Образ",
    "Состояние",
    "Статус",
    "Health",
    "Рестартов",
    "Код выхода",
    "Compose",
    "Порты",
];

impl Section for DockerSection {
    fn id(&self) -> SectionId {
        SectionId::Docker
    }

    fn is_available(&self, ctx: &DocContext<'_>) -> bool {
        snapshot(ctx).is_some()
    }

    fn human(&self, out: &mut String, ctx: &DocContext<'_>) {
        let Some(snapshot) = snapshot(ctx) else {
            return;
        };
        heading(out, &format!("Docker ({})", snapshot.version));
        out.push_str(&format!(
            "Контейнеров {} (работают {}, остановлены {}), образов {} ({}), томов {}, сетей {}.\n\n",
            snapshot.containers.len(),
            snapshot.running_count(),
            snapshot.stopped_count(),
            snapshot.images.len(),
            bytes(snapshot.images_size()),
            snapshot.volumes,
            snapshot.networks.len()
        ));
        table(out, &HEADERS_HUMAN, &rows(snapshot));
    }

    fn llm(&self, out: &mut String, ctx: &DocContext<'_>) {
        let Some(snapshot) = snapshot(ctx) else {
            return;
        };
        heading(out, "Docker");
        field(out, "version", &snapshot.version);
        field(out, "running", snapshot.running_count().to_string());
        field(out, "stopped", snapshot.stopped_count().to_string());
        field(out, "images", snapshot.images.len().to_string());
        field(
            out,
            "compose_projects",
            snapshot.compose_projects().join(", "),
        );
        field(
            out,
            "containers",
            "name | image | state | status | health | restarts | exit_code | compose | ports",
        );
        list(out, "", &rows(snapshot));
        blank(out);
    }
}
