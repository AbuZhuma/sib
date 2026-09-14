use std::fmt::Write;

use asiba_core::ServerState;
use asiba_modules::deploy::{self, DeployState, DeployStatus};

const MAX_DEPLOYS: usize = 10;

pub fn deploy(out: &mut String, server: &ServerState) {
    let Some(state) = server.data::<DeployState>(deploy::ID) else {
        return;
    };
    let snapshot = &state.snapshot;
    if snapshot.deploys.is_empty() && snapshot.runner.is_none() {
        return;
    }
    let _ = writeln!(out, "## Деплой\n");
    if let Some(runner) = &snapshot.runner {
        let state = if runner.is_busy {
            "выполняет job"
        } else {
            "свободен"
        };
        let _ = writeln!(
            out,
            "{} в `{}`: {state}\n",
            runner.kind.label(),
            runner.working_dir
        );
    }
    let _ = writeln!(
        out,
        "| Статус | Проект | Источник | Начало | Стадии | Детали |"
    );
    let _ = writeln!(out, "|---|---|---|---|---|---|");
    for deploy in snapshot.deploys.iter().take(MAX_DEPLOYS) {
        let status = match deploy.status {
            DeployStatus::InProgress => "выполняется",
            DeployStatus::Success => "успешно",
            DeployStatus::Failed => "ошибка",
        };
        let stages: Vec<&str> = deploy.stages.iter().map(|s| s.name.as_str()).collect();
        let detail = match &deploy.error {
            Some(error) => format!("{} — {}", deploy.detail, error.line),
            None => deploy.detail.clone(),
        };
        let _ = writeln!(
            out,
            "| {status} | {} | {} | {} | {} | {} |",
            deploy.project,
            deploy.source.label(),
            deploy.started_at.format("%Y-%m-%d %H:%M"),
            stages.join(" → "),
            detail.replace('|', "/")
        );
    }
    let _ = writeln!(out);
}
