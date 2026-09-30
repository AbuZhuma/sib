use sib_modules::anomalies::{self, AnomaliesSnapshot};
use sib_modules::network::{self, NetworkSnapshot};

use crate::audit::context::{AuditContext, list_or};
use crate::audit::evidence::{EvidenceSource, TAB_ANOMALIES, TAB_NETWORK};
use crate::audit::pattern::{Area, Pattern, Verdict, Weight};

const ERRORS_WARN: u64 = 1;
const ERRORS_FAIL: u64 = 1000;
const NO_DATA: &str = "the module has not collected data";

const ADVICE_ERRORS: &str = "Check the cable, the driver, the MTU and the load on the interface.";
const ADVICE_ATTACK: &str =
    "Open the Anomalies tab, ban the source addresses, limit the rate of new connections.";

pub static PATTERNS: &[Pattern] = &[
    Pattern {
        id: "network.interface_errors",
        area: Area::Network,
        subject: "Interface errors and drops",
        description: "Error and dropped packet counters per interface since boot.",
        weight: Weight::Low,
        advice: ADVICE_ERRORS,
        evidence: EvidenceSource::Tab(TAB_NETWORK),
        evaluate: interface_errors,
    },
    Pattern {
        id: "network.attack_signs",
        area: Area::Network,
        subject: "Signs of DDoS and scanning",
        description: "Attack signs from the anomalies module: SYN flood, a burst of connections from one address, packets per second over the threshold.",
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
        list_or(&noisy, "no errors"),
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
        list_or(&signs, "traffic looks normal"),
    )
    .single()
}
