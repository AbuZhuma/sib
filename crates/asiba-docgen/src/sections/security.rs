use asiba_modules::security::{
    self, Category, Check, CheckStatus, FirewallState, SecuritySnapshot, Weight,
};

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

fn status_name(status: CheckStatus) -> &'static str {
    match status {
        CheckStatus::Pass => "pass",
        CheckStatus::Warn => "warn",
        CheckStatus::Fail => "FAIL",
        CheckStatus::Unknown => "unknown",
    }
}

fn weight_name(weight: Weight) -> &'static str {
    match weight {
        Weight::Low => "low",
        Weight::Medium => "medium",
        Weight::High => "high",
    }
}

fn check_rows(checks: &[Check], category: Category) -> Vec<Vec<String>> {
    checks
        .iter()
        .filter(|c| c.category == category)
        .map(|c| {
            vec![
                status_name(c.status).to_owned(),
                weight_name(c.weight).to_owned(),
                c.label.to_owned(),
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

fn overview(snapshot: &SecuritySnapshot) -> Vec<(&'static str, &'static str, String)> {
    let checks = security::checks(snapshot);
    let score = security::score(&checks);
    vec![
        (
            "Оценка",
            "score",
            format!(
                "{} ({}%, {} of {} checks passed, {} high-weight failures)",
                score.grade.letter(),
                score.percent,
                score.passed,
                score.known,
                score.failed_high
            ),
        ),
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
        heading(out, "Безопасность");
        for (label, _, value) in overview(snapshot) {
            bullet(out, label, value);
        }
        blank(out);
        let checks = security::checks(snapshot);
        for category in Category::ALL {
            let rows = check_rows(&checks, category);
            if rows.is_empty() {
                continue;
            }
            subheading(out, category.label());
            table(out, &["Статус", "Вес", "Проверка", "Детали"], &rows);
        }
        subheading(out, "Атакующие IP");
        table(
            out,
            &[
                "IP",
                "Страна",
                "Ошибок",
                "За 10 мин",
                "Логины",
                "Последняя",
                "Пометки",
            ],
            &attacker_rows(snapshot, ctx),
        );
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

    fn llm(&self, out: &mut String, ctx: &DocContext<'_>) {
        let Some(snapshot) = snapshot(ctx) else {
            return;
        };
        heading(out, "Security");
        for (_, key, value) in overview(snapshot) {
            field(out, key, value);
        }
        let checks = security::checks(snapshot);
        field(out, "checks", "status | weight | check | detail");
        for category in Category::ALL {
            let rows = check_rows(&checks, category);
            if rows.is_empty() {
                continue;
            }
            field(out, "category", category.label());
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
