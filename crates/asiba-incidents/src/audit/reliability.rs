use asiba_core::ServerState;
use asiba_modules::deploy::{self, DeployState, DeployStatus};
use asiba_modules::docker::{self, DockerSnapshot};
use asiba_modules::services::{self, ServicesSnapshot};

use super::{Area, AuditCheck};

const RESTARTS_WARN: u32 = 1;
const RESTARTS_FAIL: u32 = 10;
const RESTART_LOOP: u32 = 5;
const DANGLING_WARN: usize = 5;
const DANGLING_FAIL: usize = 30;

const ADVICE_FAILED_UNITS: &str = "Смотрите journalctl -u <юнит> (кнопка «журнал» на вкладке «Сервисы»), исправьте причину и перезапустите.";
const ADVICE_RESTARTS: &str =
    "Юнит падает и поднимается - проверьте его журнал; частые рестарты означают скрытую ошибку.";
const ADVICE_CONTAINERS_DOWN: &str =
    "Посмотрите docker logs (кнопка «логи») и код выхода; поднимите контейнер после исправления.";
const ADVICE_UNHEALTHY: &str =
    "Healthcheck контейнера не проходит - проверьте зависимости (БД, сеть) и логи.";
const ADVICE_RESTART_LOOP: &str =
    "Контейнер в петле перезапусков - логи покажут ошибку при старте.";
const ADVICE_NO_POLICY: &str = "Запущенные вручную контейнеры без restart policy не поднимутся после перезагрузки - задайте --restart unless-stopped или compose.";
const ADVICE_DANGLING: &str = "Старые слои занимают диск - docker image prune.";
const ADVICE_DEPLOYS: &str =
    "Откройте вкладку «Деплой»: там хвост лога и стадия, на которой всё упало.";

pub fn checks(server: &ServerState) -> Vec<AuditCheck> {
    let mut checks = Vec::new();
    if let Some(snapshot) = server.data::<ServicesSnapshot>(services::ID) {
        checks.extend(unit_checks(snapshot));
    }
    if let Some(snapshot) = server.data::<DockerSnapshot>(docker::ID) {
        checks.extend(container_checks(snapshot));
    }
    if let Some(state) = server.data::<DeployState>(deploy::ID) {
        checks.push(deploys(state));
    }
    checks
}

fn unit_checks(snapshot: &ServicesSnapshot) -> Vec<AuditCheck> {
    let failed: Vec<&str> = snapshot.failed().map(|u| u.name.as_str()).collect();
    let restarting: Vec<String> = snapshot
        .units
        .iter()
        .filter(|u| u.restarts >= RESTARTS_WARN)
        .map(|u| format!("{} ({})", u.name, u.restarts))
        .collect();
    let max_restarts = snapshot.units.iter().map(|u| u.restarts).max().unwrap_or(0);
    vec![
        AuditCheck::new(
            Area::Reliability,
            "Нет упавших юнитов systemd",
            ADVICE_FAILED_UNITS,
        )
        .graded(
            !failed.is_empty(),
            false,
            list_or(&failed, "все юниты в порядке"),
        ),
        AuditCheck::new(
            Area::Reliability,
            "Юниты не перезапускаются",
            ADVICE_RESTARTS,
        )
        .graded(
            max_restarts >= RESTARTS_FAIL,
            !restarting.is_empty(),
            list_or(&restarting, "рестартов нет"),
        ),
    ]
}

fn container_checks(snapshot: &DockerSnapshot) -> Vec<AuditCheck> {
    let containers = &snapshot.containers;
    let down: Vec<String> = containers
        .iter()
        .filter(|c| !c.is_running() && (c.exit_code != 0 || should_run(&c.restart_policy)))
        .map(|c| format!("{} (код {})", c.name, c.exit_code))
        .collect();
    let unhealthy: Vec<&str> = containers
        .iter()
        .filter(|c| c.is_running() && c.is_unhealthy())
        .map(|c| c.name.as_str())
        .collect();
    let looping: Vec<String> = containers
        .iter()
        .filter(|c| c.restart_count >= RESTART_LOOP)
        .map(|c| format!("{} ({})", c.name, c.restart_count))
        .collect();
    let no_policy: Vec<&str> = containers
        .iter()
        .filter(|c| c.is_running() && !should_run(&c.restart_policy))
        .map(|c| c.name.as_str())
        .collect();
    let dangling = snapshot.images.iter().filter(|i| i.is_dangling()).count();
    vec![
        AuditCheck::new(
            Area::Reliability,
            "Контейнеры, которые должны работать, запущены",
            ADVICE_CONTAINERS_DOWN,
        )
        .graded(!down.is_empty(), false, list_or(&down, "все запущены")),
        AuditCheck::new(
            Area::Reliability,
            "Контейнеры проходят healthcheck",
            ADVICE_UNHEALTHY,
        )
        .graded(
            false,
            !unhealthy.is_empty(),
            list_or(&unhealthy, "unhealthy нет"),
        ),
        AuditCheck::new(
            Area::Reliability,
            "Нет петли перезапусков контейнеров",
            ADVICE_RESTART_LOOP,
        )
        .graded(false, !looping.is_empty(), list_or(&looping, "петель нет")),
        AuditCheck::new(
            Area::Reliability,
            "У контейнеров задана политика перезапуска",
            ADVICE_NO_POLICY,
        )
        .graded(
            false,
            !no_policy.is_empty(),
            list_or(&no_policy, "у всех задана"),
        ),
        AuditCheck::new(
            Area::Reliability,
            "Нет висячих образов Docker",
            ADVICE_DANGLING,
        )
        .graded(
            dangling >= DANGLING_FAIL,
            dangling >= DANGLING_WARN,
            format!("{dangling} dangling-образов"),
        ),
    ]
}

fn should_run(policy: &str) -> bool {
    matches!(policy, "always" | "unless-stopped" | "on-failure")
}

fn deploys(state: &DeployState) -> AuditCheck {
    let failed: Vec<&str> = state
        .snapshot
        .deploys
        .iter()
        .filter(|d| d.status == DeployStatus::Failed)
        .map(|d| d.project.as_str())
        .collect();
    AuditCheck::new(
        Area::Reliability,
        "Последние деплои успешны",
        ADVICE_DEPLOYS,
    )
    .graded(
        !failed.is_empty(),
        false,
        list_or(&failed, "упавших деплоев нет"),
    )
}

pub fn list_or<T: AsRef<str>>(items: &[T], empty: &str) -> String {
    if items.is_empty() {
        return empty.to_owned();
    }
    items
        .iter()
        .map(AsRef::as_ref)
        .collect::<Vec<_>>()
        .join(", ")
}
