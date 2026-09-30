use sib_modules::deploy::{self, DeployState, DeployStatus};
use sib_modules::docker::{self, DockerSnapshot};
use sib_modules::services::{self, ServicesSnapshot};

use crate::audit::context::{AuditContext, list_or};
use crate::audit::evidence::{EvidenceSource, TAB_DEPLOY, TAB_DOCKER, TAB_SERVICES};
use crate::audit::pattern::{Area, Pattern, Verdict, Weight};

const RESTARTS_WARN: u32 = 1;
const RESTARTS_FAIL: u32 = 10;
const RESTART_LOOP: u32 = 5;
const DANGLING_WARN: usize = 5;
const DANGLING_FAIL: usize = 30;
const NO_DATA: &str = "модуль не собрал данные";

const ADVICE_FAILED_UNITS: &str = "Откройте journalctl -u <юнит> (кнопка «журнал» на вкладке «Сервисы»), устраните причину и перезапустите юнит.";
const ADVICE_RESTARTS: &str =
    "Проверьте журнал юнита: частые перезапуски означают повторяющуюся ошибку.";
const ADVICE_CONTAINERS_DOWN: &str =
    "Посмотрите docker logs (кнопка «логи») и код выхода, затем запустите контейнер.";
const ADVICE_UNHEALTHY: &str = "Проверьте зависимости контейнера (база данных, сеть) и его логи.";
const ADVICE_RESTART_LOOP: &str = "Ошибка видна в логах контейнера при старте.";
const ADVICE_NO_POLICY: &str = "Задайте --restart unless-stopped или опишите контейнер в compose.";
const ADVICE_DANGLING: &str = "Старые слои занимают диск - docker image prune.";
const ADVICE_DEPLOYS: &str =
    "Откройте вкладку «Деплой»: там стадия, на которой деплой остановился, и хвост лога.";

pub static PATTERNS: &[Pattern] = &[
    Pattern {
        id: "reliability.failed_units",
        area: Area::Reliability,
        subject: "Упавшие юниты systemd",
        description: "Юниты в состоянии failed: сервис не запустился или упал и не был перезапущен.",
        weight: Weight::High,
        advice: ADVICE_FAILED_UNITS,
        evidence: EvidenceSource::Tab(TAB_SERVICES),
        evaluate: failed_units,
    },
    Pattern {
        id: "reliability.unit_restarts",
        area: Area::Reliability,
        subject: "Перезапуски юнитов",
        description: "Счётчик NRestarts: сколько раз systemd поднимал юнит после падения.",
        weight: Weight::Medium,
        advice: ADVICE_RESTARTS,
        evidence: EvidenceSource::Tab(TAB_SERVICES),
        evaluate: unit_restarts,
    },
    Pattern {
        id: "reliability.containers_down",
        area: Area::Reliability,
        subject: "Остановленные контейнеры",
        description: "Контейнеры, которые должны работать (по restart policy или завершились с ошибкой), но не запущены.",
        weight: Weight::High,
        advice: ADVICE_CONTAINERS_DOWN,
        evidence: EvidenceSource::Tab(TAB_DOCKER),
        evaluate: containers_down,
    },
    Pattern {
        id: "reliability.unhealthy",
        area: Area::Reliability,
        subject: "Healthcheck контейнеров",
        description: "Запущенные контейнеры, чей healthcheck возвращает unhealthy.",
        weight: Weight::Medium,
        advice: ADVICE_UNHEALTHY,
        evidence: EvidenceSource::Tab(TAB_DOCKER),
        evaluate: unhealthy,
    },
    Pattern {
        id: "reliability.restart_loop",
        area: Area::Reliability,
        subject: "Петля перезапусков контейнеров",
        description: "Контейнер с большим RestartCount падает сразу после старта и поднимается снова.",
        weight: Weight::Medium,
        advice: ADVICE_RESTART_LOOP,
        evidence: EvidenceSource::Tab(TAB_DOCKER),
        evaluate: restart_loop,
    },
    Pattern {
        id: "reliability.restart_policy",
        area: Area::Reliability,
        subject: "Политика перезапуска контейнеров",
        description: "Контейнеры без restart policy не поднимутся после перезагрузки сервера.",
        weight: Weight::Low,
        advice: ADVICE_NO_POLICY,
        evidence: EvidenceSource::Tab(TAB_DOCKER),
        evaluate: restart_policy,
    },
    Pattern {
        id: "reliability.dangling_images",
        area: Area::Reliability,
        subject: "Висячие образы Docker",
        description: "Образы без тега, оставшиеся после сборок и pull. Занимают место на диске.",
        weight: Weight::Low,
        advice: ADVICE_DANGLING,
        evidence: EvidenceSource::Tab(TAB_DOCKER),
        evaluate: dangling_images,
    },
    Pattern {
        id: "reliability.deploys",
        area: Area::Reliability,
        subject: "Последние деплои",
        description: "Деплои со статусом failed в истории модуля deploy.",
        weight: Weight::Medium,
        advice: ADVICE_DEPLOYS,
        evidence: EvidenceSource::Tab(TAB_DEPLOY),
        evaluate: deploys,
    },
];

fn failed_units(ctx: &AuditContext<'_>) -> Vec<Verdict> {
    let Some(snapshot) = ctx.data::<ServicesSnapshot>(services::ID) else {
        return Verdict::skipped(NO_DATA).single();
    };
    let failed: Vec<&str> = snapshot.failed().map(|u| u.name.as_str()).collect();
    Verdict::graded(
        !failed.is_empty(),
        false,
        list_or(&failed, "все юниты в порядке"),
    )
    .single()
}

fn unit_restarts(ctx: &AuditContext<'_>) -> Vec<Verdict> {
    let Some(snapshot) = ctx.data::<ServicesSnapshot>(services::ID) else {
        return Verdict::skipped(NO_DATA).single();
    };
    let restarting: Vec<String> = snapshot
        .units
        .iter()
        .filter(|u| u.restarts >= RESTARTS_WARN)
        .map(|u| format!("{} ({})", u.name, u.restarts))
        .collect();
    let max_restarts = snapshot.units.iter().map(|u| u.restarts).max().unwrap_or(0);
    Verdict::graded(
        max_restarts >= RESTARTS_FAIL,
        !restarting.is_empty(),
        list_or(&restarting, "рестартов нет"),
    )
    .single()
}

fn should_run(policy: &str) -> bool {
    matches!(policy, "always" | "unless-stopped" | "on-failure")
}

fn containers_down(ctx: &AuditContext<'_>) -> Vec<Verdict> {
    let Some(snapshot) = ctx.data::<DockerSnapshot>(docker::ID) else {
        return Verdict::skipped(NO_DATA).single();
    };
    let down: Vec<String> = snapshot
        .containers
        .iter()
        .filter(|c| !c.is_running() && (c.exit_code != 0 || should_run(&c.restart_policy)))
        .map(|c| format!("{} (код {})", c.name, c.exit_code))
        .collect();
    Verdict::graded(!down.is_empty(), false, list_or(&down, "все запущены")).single()
}

fn unhealthy(ctx: &AuditContext<'_>) -> Vec<Verdict> {
    let Some(snapshot) = ctx.data::<DockerSnapshot>(docker::ID) else {
        return Verdict::skipped(NO_DATA).single();
    };
    let unhealthy: Vec<&str> = snapshot
        .containers
        .iter()
        .filter(|c| c.is_running() && c.is_unhealthy())
        .map(|c| c.name.as_str())
        .collect();
    Verdict::graded(
        false,
        !unhealthy.is_empty(),
        list_or(&unhealthy, "unhealthy нет"),
    )
    .single()
}

fn restart_loop(ctx: &AuditContext<'_>) -> Vec<Verdict> {
    let Some(snapshot) = ctx.data::<DockerSnapshot>(docker::ID) else {
        return Verdict::skipped(NO_DATA).single();
    };
    let looping: Vec<String> = snapshot
        .containers
        .iter()
        .filter(|c| c.restart_count >= RESTART_LOOP)
        .map(|c| format!("{} ({})", c.name, c.restart_count))
        .collect();
    Verdict::graded(false, !looping.is_empty(), list_or(&looping, "петель нет")).single()
}

fn restart_policy(ctx: &AuditContext<'_>) -> Vec<Verdict> {
    let Some(snapshot) = ctx.data::<DockerSnapshot>(docker::ID) else {
        return Verdict::skipped(NO_DATA).single();
    };
    let no_policy: Vec<&str> = snapshot
        .containers
        .iter()
        .filter(|c| c.is_running() && !should_run(&c.restart_policy))
        .map(|c| c.name.as_str())
        .collect();
    Verdict::graded(
        false,
        !no_policy.is_empty(),
        list_or(&no_policy, "у всех задана"),
    )
    .single()
}

fn dangling_images(ctx: &AuditContext<'_>) -> Vec<Verdict> {
    let Some(snapshot) = ctx.data::<DockerSnapshot>(docker::ID) else {
        return Verdict::skipped(NO_DATA).single();
    };
    let dangling = snapshot.images.iter().filter(|i| i.is_dangling()).count();
    Verdict::graded(
        dangling >= DANGLING_FAIL,
        dangling >= DANGLING_WARN,
        format!("{dangling} dangling-образов"),
    )
    .single()
}

fn deploys(ctx: &AuditContext<'_>) -> Vec<Verdict> {
    let Some(state) = ctx.data::<DeployState>(deploy::ID) else {
        return Verdict::skipped(NO_DATA).single();
    };
    let failed: Vec<&str> = state
        .snapshot
        .deploys
        .iter()
        .filter(|d| d.status == DeployStatus::Failed)
        .map(|d| d.project.as_str())
        .collect();
    Verdict::graded(
        !failed.is_empty(),
        false,
        list_or(&failed, "упавших деплоев нет"),
    )
    .single()
}
