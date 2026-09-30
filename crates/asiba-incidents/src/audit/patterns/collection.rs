use asiba_core::{Availability, SudoMode};

use crate::audit::context::{AuditContext, list_or};
use crate::audit::evidence::{EvidenceSource, TAB_SUMMARY};
use crate::audit::pattern::{Area, Pattern, Verdict, Weight};

const ADVICE_MODULES: &str = "Смотрите текст ошибки в таблице модулей в сводке сервера.";
const ADVICE_PARTIAL: &str = "Настройте sudo или добавьте пользователя в нужные группы.";
const ADVICE_SUDO: &str = "Задайте sudo в настройках сервера.";

pub static PATTERNS: &[Pattern] = &[
    Pattern {
        id: "collection.module_errors",
        area: Area::Collection,
        subject: "Ошибки сбора модулей",
        description: "Модули, чей последний сбор завершился ошибкой; их данные в аудите устарели или отсутствуют.",
        weight: Weight::Low,
        advice: ADVICE_MODULES,
        evidence: EvidenceSource::Tab(TAB_SUMMARY),
        evaluate: module_errors,
    },
    Pattern {
        id: "collection.partial",
        area: Area::Collection,
        subject: "Неполные данные модулей",
        description: "Модули, которым не хватает прав на часть данных.",
        weight: Weight::Low,
        advice: ADVICE_PARTIAL,
        evidence: EvidenceSource::Tab(TAB_SUMMARY),
        evaluate: partial,
    },
    Pattern {
        id: "collection.root",
        area: Area::Collection,
        subject: "Права root для проверок",
        description: "Без root часть проверок безопасности пропускается: sshd -T, /etc/shadow, правила файрвола.",
        weight: Weight::Low,
        advice: ADVICE_SUDO,
        evidence: EvidenceSource::Tab(TAB_SUMMARY),
        evaluate: root,
    },
];

fn module_errors(ctx: &AuditContext<'_>) -> Vec<Verdict> {
    let failing: Vec<String> = ctx
        .server
        .modules
        .iter()
        .filter_map(|(id, state)| state.last_error.as_ref().map(|e| format!("{}: {e}", id.0)))
        .collect();
    Verdict::graded(false, !failing.is_empty(), list_or(&failing, "ошибок нет")).single()
}

fn partial(ctx: &AuditContext<'_>) -> Vec<Verdict> {
    let partial: Vec<String> = ctx
        .server
        .modules
        .iter()
        .filter_map(|(id, state)| match &state.availability {
            Availability::Partial { missing } => Some(format!("{}: {}", id.0, missing.join(", "))),
            _ => None,
        })
        .collect();
    Verdict::graded(
        false,
        !partial.is_empty(),
        list_or(&partial, "ограничений нет"),
    )
    .single()
}

fn root(ctx: &AuditContext<'_>) -> Vec<Verdict> {
    let spec = &ctx.server.spec;
    let has_sudo = spec.sudo != SudoMode::None || spec.user == "root";
    Verdict::graded(
        false,
        !has_sudo,
        format!("пользователь {}, sudo {:?}", spec.user, spec.sudo),
    )
    .single()
}
