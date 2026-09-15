use asiba_modules::deploy::{self, Deploy, DeployState, DeployStatus};

use crate::section::{DocContext, Section, SectionId};
use crate::write::{NONE, blank, field, heading, line, list, local_time, table};

pub struct DeploySection;

const MAX_DEPLOYS: usize = 10;
const MAX_COMPOSE_EVENTS: usize = 15;
const MAX_LOG_TAIL: usize = 20;

fn state<'a>(ctx: &'a DocContext<'_>) -> Option<&'a DeployState> {
    ctx.server.data::<DeployState>(deploy::ID)
}

fn status_label(status: DeployStatus, english: bool) -> &'static str {
    match (status, english) {
        (DeployStatus::InProgress, false) => "выполняется",
        (DeployStatus::Success, false) => "успешно",
        (DeployStatus::Failed, false) => "ошибка",
        (DeployStatus::InProgress, true) => "in_progress",
        (DeployStatus::Success, true) => "success",
        (DeployStatus::Failed, true) => "failed",
    }
}

fn row(deploy: &Deploy, english: bool) -> Vec<String> {
    let stages: Vec<&str> = deploy.stages.iter().map(|s| s.name.as_str()).collect();
    let detail = match &deploy.error {
        Some(error) => format!("{} - {}", deploy.detail, error.line),
        None => deploy.detail.clone(),
    };
    vec![
        status_label(deploy.status, english).to_owned(),
        deploy.project.clone(),
        deploy.source.label().to_owned(),
        local_time(deploy.started_at),
        deploy
            .finished_at
            .map(local_time)
            .unwrap_or_else(|| NONE.to_owned()),
        stages.join(" → "),
        detail,
    ]
}

fn rows(state: &DeployState, english: bool) -> Vec<Vec<String>> {
    state
        .snapshot
        .deploys
        .iter()
        .take(MAX_DEPLOYS)
        .map(|d| row(d, english))
        .collect()
}

fn compose_rows(state: &DeployState) -> Vec<Vec<String>> {
    state
        .compose_events
        .iter()
        .rev()
        .take(MAX_COMPOSE_EVENTS)
        .map(|e| {
            vec![
                local_time(e.at),
                e.project.clone(),
                e.name.clone(),
                format!("{} {}", e.kind, e.action),
                e.exit_code
                    .map(|c| c.to_string())
                    .unwrap_or_else(|| NONE.to_owned()),
            ]
        })
        .collect()
}

fn runner_label(state: &DeployState) -> Option<String> {
    let runner = state.snapshot.runner.as_ref()?;
    let busy = if runner.is_busy { "busy" } else { "idle" };
    Some(format!(
        "{} in {} ({busy})",
        runner.kind.label(),
        runner.working_dir,
    ))
}

fn failed_log_tail(state: &DeployState) -> Option<(&str, &[String])> {
    let deploy = state
        .snapshot
        .deploys
        .iter()
        .find(|d| d.status == DeployStatus::Failed)?;
    if deploy.log_tail.is_empty() {
        return None;
    }
    let start = deploy.log_tail.len().saturating_sub(MAX_LOG_TAIL);
    Some((&deploy.project, &deploy.log_tail[start..]))
}

impl Section for DeploySection {
    fn id(&self) -> SectionId {
        SectionId::Deploy
    }

    fn is_available(&self, ctx: &DocContext<'_>) -> bool {
        state(ctx).is_some_and(|s| !s.snapshot.deploys.is_empty() || s.snapshot.runner.is_some())
    }

    fn human(&self, out: &mut String, ctx: &DocContext<'_>) {
        let Some(state) = state(ctx) else {
            return;
        };
        heading(out, "Деплой");
        if let Some(runner) = runner_label(state) {
            line(out, format!("Раннер: {runner}\n"));
        }
        table(
            out,
            &[
                "Статус",
                "Проект",
                "Источник",
                "Начало",
                "Конец",
                "Стадии",
                "Детали",
            ],
            &rows(state, false),
        );
        if !state.compose_events.is_empty() {
            line(out, "Последние события compose:\n");
            table(
                out,
                &["Время", "Проект", "Контейнер", "Событие", "Код"],
                &compose_rows(state),
            );
        }
    }

    fn llm(&self, out: &mut String, ctx: &DocContext<'_>) {
        let Some(state) = state(ctx) else {
            return;
        };
        heading(out, "Deploys");
        if let Some(runner) = runner_label(state) {
            field(out, "runner", runner);
        }
        field(
            out,
            "failed_recent",
            state.snapshot.failed_count().to_string(),
        );
        field(
            out,
            "deploys",
            "status | project | source | started | finished | stages | detail",
        );
        list(out, "", &rows(state, true));
        if !state.compose_events.is_empty() {
            field(
                out,
                "compose_events",
                "time | project | container | event | exit_code",
            );
            list(out, "", &compose_rows(state));
        }
        if let Some((project, tail)) = failed_log_tail(state) {
            field(out, "failed_deploy_log_tail", project);
            for entry in tail {
                line(out, format!("  {entry}"));
            }
            blank(out);
        }
    }
}
