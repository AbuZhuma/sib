use sib_incidents::{Area, AuditCheck, Outcome, SystemAudit, Weight, security_score, system_audit};
use sib_modules::security::{self, FirewallState, SecuritySnapshot};

use crate::section::{DocContext, Section, SectionId};
use crate::write::{NONE, blank, bullet, field, heading, list, local_time, subheading, table};

pub struct SecuritySection;

const MAX_ATTACKERS: usize = 10;
const MAX_BANS: usize = 15;
const MAX_LOGINS: usize = 10;
const MAX_SUDO: usize = 10;

fn snapshot<'a>(ctx: &'a DocContext<'_>) -> Option<&'a SecuritySnapshot> {
    ctx.server.data::<SecuritySnapshot>(security::ID)
}

fn status_name(outcome: Outcome) -> &'static str {
    match outcome {
        Outcome::Pass => "pass",
        Outcome::Warn => "warn",
        Outcome::Fail => "FAIL",
        Outcome::Skipped => "skipped",
    }
}

fn weight_name(weight: Weight) -> &'static str {
    match weight {
        Weight::Low => "low",
        Weight::Medium => "medium",
        Weight::High => "high",
    }
}

fn security_areas() -> impl Iterator<Item = Area> {
    Area::ALL.into_iter().filter(|a| a.is_security())
}

fn check_rows(audit: &SystemAudit, area: Area) -> Vec<Vec<String>> {
    audit
        .in_area(area)
        .map(|c: &AuditCheck| {
            vec![
                status_name(c.outcome).to_owned(),
                weight_name(c.weight).to_owned(),
                c.title(),
                c.detail.clone(),
            ]
        })
        .collect()
}

fn attacker_rows(snapshot: &SecuritySnapshot, ctx: &DocContext<'_>) -> Vec<Vec<String>> {
    snapshot
        .attackers
        .iter()
        .take(MAX_ATTACKERS)
        .map(|a| {
            let country = ctx.state.country_of(&a.ip).unwrap_or(NONE);
            let flag = if a.is_brute_force() {
                "brute-force"
            } else {
                ""
            };
            let banned = if snapshot.is_banned(&a.ip) {
                "banned"
            } else {
                ""
            };
            vec![
                a.ip.clone(),
                country.to_owned(),
                a.failures.to_string(),
                a.recent_failures.to_string(),
                a.users_label(),
                local_time(a.last_at),
                format!("{flag} {banned}").trim().to_owned(),
            ]
        })
        .collect()
}

fn ban_rows(snapshot: &SecuritySnapshot) -> Vec<Vec<String>> {
    snapshot
        .bans
        .iter()
        .take(MAX_BANS)
        .map(|b| {
            vec![
                b.ip.clone(),
                b.source.clone(),
                b.expires.clone().unwrap_or_else(|| "permanent".to_owned()),
            ]
        })
        .collect()
}

fn login_rows(snapshot: &SecuritySnapshot) -> Vec<Vec<String>> {
    snapshot
        .logins
        .iter()
        .rev()
        .take(MAX_LOGINS)
        .map(|l| {
            vec![
                local_time(l.at),
                l.user.clone(),
                l.from.clone(),
                l.method.clone(),
            ]
        })
        .collect()
}

fn sudo_rows(snapshot: &SecuritySnapshot) -> Vec<Vec<String>> {
    snapshot
        .sudo_calls
        .iter()
        .rev()
        .take(MAX_SUDO)
        .map(|s| {
            let result = if s.is_success { "ok" } else { "DENIED" };
            vec![
                local_time(s.at),
                s.user.clone(),
                s.target_user.clone(),
                result.to_owned(),
                s.command.clone(),
            ]
        })
        .collect()
}

fn firewall_label(snapshot: &SecuritySnapshot) -> String {
    match &snapshot.firewall {
        FirewallState::Active(backend) => format!("active ({backend})"),
        FirewallState::Inactive => "inactive".to_owned(),
        FirewallState::Unknown => "unknown".to_owned(),
    }
}

fn score_label(audit: &SystemAudit) -> String {
    let score = security_score(audit.checks.iter());
    format!(
        "{} ({}%, {} of {} checks passed, {} high-weight failures)",
        score.grade.letter(),
        score.percent,
        score.passed,
        score.known,
        score.failed_high
    )
}

fn overview(
    snapshot: &SecuritySnapshot,
    audit: &SystemAudit,
) -> Vec<(&'static str, &'static str, String)> {
    vec![
        ("Оценка", "score", score_label(audit)),
        (
            "Неудачных входов за 24 ч",
            "failed_logins_24h",
            snapshot.failed_logins.to_string(),
        ),
        (
            "Атакующих сейчас",
            "brute_force_now",
            snapshot.brute_force_count().to_string(),
        ),
        ("Банов", "bans", snapshot.bans.len().to_string()),
        (
            "Механизм бана",
            "ban_backend",
            snapshot
                .ban_backend
                .map(|b| b.label().to_owned())
                .unwrap_or_else(|| "none".to_owned()),
        ),
        ("Файрвол", "firewall", firewall_label(snapshot)),
        (
            "fail2ban",
            "fail2ban",
            if snapshot.has_fail2ban { "yes" } else { "no" }.to_owned(),
        ),
        (
            "Данные с sudo",
            "root_view",
            snapshot.is_root_view.to_string(),
        ),
    ]
}

fn human_checks(out: &mut String, audit: &SystemAudit) {
    for area in security_areas() {
        let rows = check_rows(audit, area);
        if rows.is_empty() {
            continue;
        }
        subheading(out, area.label());
        table(out, &["Статус", "Вес", "Проверка", "Детали"], &rows);
    }
}

fn human_activity(out: &mut String, snapshot: &SecuritySnapshot, ctx: &DocContext<'_>) {
    subheading(out, "Атакующие IP");
    let attacker_columns = [
        "IP",
        "Страна",
        "Ошибок",
        "За 10 мин",
        "Логины",
        "Последняя",
        "Пометки",
    ];
    table(out, &attacker_columns, &attacker_rows(snapshot, ctx));
    subheading(out, "Баны");
    table(out, &["IP", "Источник", "Истекает"], &ban_rows(snapshot));
    subheading(out, "Последние входы");
    table(
        out,
        &["Время", "Пользователь", "Откуда", "Метод"],
        &login_rows(snapshot),
    );
    subheading(out, "Вызовы sudo");
    table(
        out,
        &["Время", "Пользователь", "Как", "Результат", "Команда"],
        &sudo_rows(snapshot),
    );
}

impl Section for SecuritySection {
    fn id(&self) -> SectionId {
        SectionId::Security
    }

    fn is_available(&self, ctx: &DocContext<'_>) -> bool {
        snapshot(ctx).is_some()
    }

    fn human(&self, out: &mut String, ctx: &DocContext<'_>) {
        let Some(snapshot) = snapshot(ctx) else {
            return;
        };
        let audit = system_audit(ctx.server, ctx.state);
        heading(out, "Безопасность");
        for (label, _, value) in overview(snapshot, &audit) {
            bullet(out, label, value);
        }
        blank(out);
        human_checks(out, &audit);
        human_activity(out, snapshot, ctx);
    }

    fn llm(&self, out: &mut String, ctx: &DocContext<'_>) {
        let Some(snapshot) = snapshot(ctx) else {
            return;
        };
        let audit = system_audit(ctx.server, ctx.state);
        heading(out, "Security");
        for (_, key, value) in overview(snapshot, &audit) {
            field(out, key, value);
        }
        field(out, "checks", "status | weight | check | detail");
        for area in security_areas() {
            let rows = check_rows(&audit, area);
            if rows.is_empty() {
                continue;
            }
            field(out, "category", area.label());
            list(out, "  ", &rows);
        }
        field(
            out,
            "attackers",
            "ip | country | failures_24h | failures_10m | users_tried | last_seen | flags",
        );
        list(out, "", &attacker_rows(snapshot, ctx));
        field(out, "bans", "ip | source | expires");
        list(out, "", &ban_rows(snapshot));
        field(out, "recent_logins", "time | user | from | method");
        list(out, "", &login_rows(snapshot));
        field(out, "sudo_calls", "time | user | as | result | command");
        list(out, "", &sudo_rows(snapshot));
        blank(out);
    }
}
