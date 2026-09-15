use asiba_core::{Availability, ServerState, SudoMode};
use asiba_modules::logs::{self, LogsSnapshot};
use asiba_modules::system::{self, SystemInfo};
use asiba_modules::updates::{self, UpdatesSnapshot};
use chrono::Duration;

use super::reliability::list_or;
use super::{Area, AuditCheck};

const CLOCK_WARN_SECS: i64 = 5;
const CLOCK_FAIL_SECS: i64 = 60;
const UPTIME_WARN_DAYS: i64 = 180;
const UPTIME_FAIL_DAYS: i64 = 365;
const PENDING_WARN: u32 = 50;
const ERRORS_HOUR_WARN: usize = 10;
const ERRORS_HOUR_FAIL: usize = 100;
const CRITICAL_PRIORITY: u8 = 2;
const ERROR_PRIORITY: u8 = 3;

const ADVICE_CLOCK: &str = "Часы разъехались - включите NTP (timedatectl set-ntp true) и проверьте chrony/systemd-timesyncd.";
const ADVICE_UPTIME: &str =
    "Долгий аптайм означает, что ядро не обновлялось - запланируйте перезагрузку после обновлений.";
const ADVICE_SECURITY_UPDATES: &str =
    "Установите обновления безопасности (apt upgrade / dnf upgrade --security).";
const ADVICE_PENDING: &str =
    "Накопилось много обновлений - обновите систему в ближайшее окно обслуживания.";
const ADVICE_REBOOT: &str = "Обновлённое ядро или libc заработают только после перезагрузки.";
const ADVICE_ERRORS: &str =
    "Смотрите вкладку «Логи» с группировкой повторов: кто пишет ошибки и почему.";
const ADVICE_CRITICAL: &str =
    "Записи уровня crit/alert/emerg - железо, файловая система или ядро; разберите каждую.";
const ADVICE_MODULES: &str =
    "Модуль не может собрать данные - смотрите ошибку в таблице модулей в сводке.";
const ADVICE_PARTIAL: &str =
    "Часть данных недоступна без прав - настройте sudo или добавьте пользователя в нужные группы.";
const ADVICE_SUDO: &str = "Без sudo недоступны проверки sshd -T, файрвола и чужих процессов - задайте sudo в настройках сервера.";

pub fn checks(server: &ServerState) -> Vec<AuditCheck> {
    let mut checks = Vec::new();
    if let Some(info) = server.data::<SystemInfo>(system::ID) {
        checks.push(clock(info));
        checks.push(uptime(info));
    }
    if let Some(snapshot) = server.data::<UpdatesSnapshot>(updates::ID) {
        checks.extend(update_checks(snapshot));
    }
    if let Some(snapshot) = server.data::<LogsSnapshot>(logs::ID) {
        checks.extend(log_checks(snapshot));
    }
    checks.extend(collection_checks(server));
    checks
}

fn clock(info: &SystemInfo) -> AuditCheck {
    let offset = info.clock_offset_secs.abs();
    AuditCheck::new(Area::Reliability, "Часы сервера синхронны", ADVICE_CLOCK).graded(
        offset >= CLOCK_FAIL_SECS,
        offset >= CLOCK_WARN_SECS,
        format!("расхождение с этой машиной {} с", info.clock_offset_secs),
    )
}

fn uptime(info: &SystemInfo) -> AuditCheck {
    let days = info.uptime.as_secs() as i64 / 86_400;
    AuditCheck::new(Area::Updates, "Аптайм не слишком долгий", ADVICE_UPTIME).graded(
        days >= UPTIME_FAIL_DAYS,
        days >= UPTIME_WARN_DAYS,
        format!("{days} дней"),
    )
}

fn update_checks(snapshot: &UpdatesSnapshot) -> Vec<AuditCheck> {
    vec![
        AuditCheck::new(
            Area::Updates,
            "Нет обновлений безопасности в ожидании",
            ADVICE_SECURITY_UPDATES,
        )
        .graded(
            snapshot.security > 0,
            false,
            format!("{} ожидают", snapshot.security),
        ),
        AuditCheck::new(Area::Updates, "Система обновлена", ADVICE_PENDING).graded(
            false,
            snapshot.pending >= PENDING_WARN,
            format!(
                "{} пакетов ожидают ({})",
                snapshot.pending,
                snapshot.manager.label()
            ),
        ),
        AuditCheck::new(Area::Updates, "Перезагрузка не требуется", ADVICE_REBOOT).graded(
            false,
            snapshot.reboot_required,
            if snapshot.reboot_required {
                "требуется перезагрузка"
            } else {
                "не требуется"
            },
        ),
    ]
}

fn log_checks(snapshot: &LogsSnapshot) -> Vec<AuditCheck> {
    let errors = snapshot.count_since(Duration::hours(1), ERROR_PRIORITY);
    let critical = snapshot.count_since(Duration::hours(24), CRITICAL_PRIORITY);
    vec![
        AuditCheck::new(Area::Logs, "Мало ошибок в журнале за час", ADVICE_ERRORS).graded(
            errors >= ERRORS_HOUR_FAIL,
            errors >= ERRORS_HOUR_WARN,
            format!("{errors} записей уровня err и выше"),
        ),
        AuditCheck::new(
            Area::Logs,
            "Нет критичных записей за сутки",
            ADVICE_CRITICAL,
        )
        .graded(
            critical > 0,
            false,
            format!("{critical} записей уровня crit и выше"),
        ),
    ]
}

fn collection_checks(server: &ServerState) -> Vec<AuditCheck> {
    let failing: Vec<String> = server
        .modules
        .iter()
        .filter_map(|(id, state)| state.last_error.as_ref().map(|e| format!("{}: {e}", id.0)))
        .collect();
    let partial: Vec<String> = server
        .modules
        .iter()
        .filter_map(|(id, state)| match &state.availability {
            Availability::Partial { missing } => Some(format!("{}: {}", id.0, missing.join(", "))),
            _ => None,
        })
        .collect();
    let has_sudo = server.spec.sudo != SudoMode::None || server.spec.user == "root";
    vec![
        AuditCheck::new(
            Area::Collection,
            "Все модули собирают данные",
            ADVICE_MODULES,
        )
        .graded(false, !failing.is_empty(), list_or(&failing, "ошибок нет")),
        AuditCheck::new(Area::Collection, "Модули видят все данные", ADVICE_PARTIAL).graded(
            false,
            !partial.is_empty(),
            list_or(&partial, "ограничений нет"),
        ),
        AuditCheck::new(
            Area::Collection,
            "Есть права root для проверок",
            ADVICE_SUDO,
        )
        .graded(
            false,
            !has_sudo,
            format!(
                "пользователь {}, sudo {:?}",
                server.spec.user, server.spec.sudo
            ),
        ),
    ]
}
