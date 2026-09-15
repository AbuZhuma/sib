use asiba_core::{AppState, ServerState};
use asiba_modules::security::{self, Category, CheckStatus, SecuritySnapshot};

use super::{Area, AuditCheck, Outcome};

const ADVICE_SSH: &str =
    "Поправьте параметр в /etc/ssh/sshd_config (или sshd_config.d/) и перезапустите sshd.";
const ADVICE_ACCESS: &str =
    "Проверьте учётные записи, sudoers и права на файлы; уберите лишние права.";
const ADVICE_NETWORK: &str = "Включите файрвол и оставьте снаружи только нужные порты.";
const ADVICE_KERNEL: &str = "Задайте параметр в /etc/sysctl.d/ и примените sysctl --system.";
const ADVICE_SYSTEM: &str = "Включите соответствующий сервис или механизм защиты.";
const ADVICE_BRUTE_FORCE: &str = "Поставьте fail2ban или забаньте адреса кнопкой на вкладке «Безопасность», отключите вход по паролю.";
const ADVICE_BANS: &str =
    "Забаньте атакующие адреса на вкладке «Безопасность» или включите fail2ban.";

fn advice(category: Category) -> &'static str {
    match category {
        Category::Ssh => ADVICE_SSH,
        Category::Access => ADVICE_ACCESS,
        Category::Network => ADVICE_NETWORK,
        Category::Kernel => ADVICE_KERNEL,
        Category::System => ADVICE_SYSTEM,
    }
}

fn outcome(status: CheckStatus) -> Outcome {
    match status {
        CheckStatus::Pass => Outcome::Pass,
        CheckStatus::Warn => Outcome::Warn,
        CheckStatus::Fail => Outcome::Fail,
        CheckStatus::Unknown => Outcome::Skipped,
    }
}

pub fn checks(server: &ServerState, state: &AppState) -> Vec<AuditCheck> {
    let Some(snapshot) = server.data::<SecuritySnapshot>(security::ID) else {
        return Vec::new();
    };
    let mut checks: Vec<AuditCheck> = security::checks(snapshot)
        .into_iter()
        .map(|check| {
            AuditCheck::new(Area::Security, check.label, advice(check.category))
                .outcome(outcome(check.status), check.detail)
        })
        .collect();
    checks.push(brute_force(snapshot, state));
    checks.push(unbanned_attackers(snapshot));
    checks
}

fn brute_force(snapshot: &SecuritySnapshot, state: &AppState) -> AuditCheck {
    let attackers: Vec<String> = snapshot
        .attackers
        .iter()
        .filter(|a| a.is_brute_force())
        .map(|a| {
            format!(
                "{} ({}, {} попыток за 10 мин)",
                a.ip,
                state.country_of(&a.ip).unwrap_or("?"),
                a.recent_failures
            )
        })
        .collect();
    let check = AuditCheck::new(
        Area::Security,
        "Нет активного брутфорса SSH",
        ADVICE_BRUTE_FORCE,
    );
    if attackers.is_empty() {
        let detail = format!(
            "{} адресов с неудачными попытками за сутки",
            snapshot.attackers.len()
        );
        return check.outcome(Outcome::Pass, detail);
    }
    check.outcome(Outcome::Fail, attackers.join("; "))
}

fn unbanned_attackers(snapshot: &SecuritySnapshot) -> AuditCheck {
    let unbanned = snapshot
        .attackers
        .iter()
        .filter(|a| a.is_brute_force() && !snapshot.is_banned(&a.ip))
        .count();
    AuditCheck::new(
        Area::Security,
        "Атакующие адреса заблокированы",
        ADVICE_BANS,
    )
    .graded(
        false,
        unbanned > 0,
        format!(
            "{unbanned} атакующих без бана, банов всего {}",
            snapshot.bans.len()
        ),
    )
}
