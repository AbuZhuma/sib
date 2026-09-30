use asiba_modules::anomalies::{self, AnomaliesSnapshot};
use asiba_modules::network::{self, NetworkSnapshot};

use crate::audit::context::{AuditContext, list_or};
use crate::audit::evidence::{EvidenceSource, TAB_ANOMALIES, TAB_NETWORK};
use crate::audit::pattern::{Area, Pattern, Verdict, Weight};

const ERRORS_WARN: u64 = 1;
const ERRORS_FAIL: u64 = 1000;
const NO_DATA: &str = "модуль не собрал данные";

const ADVICE_ERRORS: &str = "Проверьте кабель, драйвер, MTU и нагрузку интерфейса.";
const ADVICE_ATTACK: &str = "Откройте вкладку «Аномалии», забаньте адреса-источники, ограничьте скорость новых подключений.";

pub static PATTERNS: &[Pattern] = &[
    Pattern {
        id: "network.interface_errors",
        area: Area::Network,
        subject: "Ошибки и дропы интерфейсов",
        description: "Счётчики ошибок и отброшенных пакетов по интерфейсам с момента загрузки.",
        weight: Weight::Low,
        advice: ADVICE_ERRORS,
        evidence: EvidenceSource::Tab(TAB_NETWORK),
        evaluate: interface_errors,
    },
    Pattern {
        id: "network.attack_signs",
        area: Area::Network,
        subject: "Признаки DDoS и сканирования",
        description: "Признаки атаки из модуля аномалий: SYN-флуд, всплеск соединений с одного адреса, превышение порога по пакетам в секунду.",
        weight: Weight::High,
        advice: ADVICE_ATTACK,
        evidence: EvidenceSource::Tab(TAB_ANOMALIES),
        evaluate: attack_signs,
    },
];

fn interface_errors(ctx: &AuditContext<'_>) -> Vec<Verdict> {
    let Some(snapshot) = ctx.data::<NetworkSnapshot>(network::ID) else {
        return Verdict::skipped(NO_DATA).single();
    };
    let counted: Vec<(&str, u64)> = snapshot
        .interfaces
        .iter()
        .filter(|i| !i.is_loopback())
        .map(|i| {
            (
                i.name.as_str(),
                i.rx.errors + i.tx.errors + i.rx.drops + i.tx.drops,
            )
        })
        .collect();
    let noisy: Vec<String> = counted
        .iter()
        .filter(|(_, errors)| *errors >= ERRORS_WARN)
        .map(|(name, errors)| format!("{name}: {errors}"))
        .collect();
    let worst = counted.iter().map(|(_, e)| *e).max().unwrap_or(0);
    Verdict::graded(
        worst >= ERRORS_FAIL,
        !noisy.is_empty(),
        list_or(&noisy, "ошибок нет"),
    )
    .single()
}

fn attack_signs(ctx: &AuditContext<'_>) -> Vec<Verdict> {
    let Some(snapshot) = ctx.data::<AnomaliesSnapshot>(anomalies::ID) else {
        return Verdict::skipped(NO_DATA).single();
    };
    let signs: Vec<String> = snapshot
        .signs
        .iter()
        .map(|s| format!("{}: {}", s.kind.label(), s.detail))
        .collect();
    Verdict::graded(
        snapshot.is_under_attack(),
        snapshot.has_signs(),
        list_or(&signs, "трафик в норме"),
    )
    .single()
}
