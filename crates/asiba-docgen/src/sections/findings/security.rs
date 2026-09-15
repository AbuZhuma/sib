use asiba_core::Severity;
use asiba_modules::anomalies::{self, AnomaliesSnapshot};
use asiba_modules::security::{self, CheckStatus, SecuritySnapshot, Weight};

use super::Finding;
use crate::section::DocContext;

pub fn collect(ctx: &DocContext<'_>, out: &mut Vec<Finding>) {
    checks(ctx, out);
    attackers(ctx, out);
    alerts(ctx, out);
    anomalies(ctx, out);
}

fn checks(ctx: &DocContext<'_>, out: &mut Vec<Finding>) {
    let Some(snapshot) = ctx.server.data::<SecuritySnapshot>(security::ID) else {
        return;
    };
    for check in security::checks(snapshot) {
        let severity = match (check.status, check.weight) {
            (CheckStatus::Fail, Weight::High) => Severity::Critical,
            (CheckStatus::Fail, _) | (CheckStatus::Warn, Weight::High) => Severity::Warning,
            (CheckStatus::Warn, _) => Severity::Info,
            _ => continue,
        };
        out.push(Finding::new(
            severity,
            "security",
            format!("{} — {}", check.label, check.detail),
        ));
    }
}

fn attackers(ctx: &DocContext<'_>, out: &mut Vec<Finding>) {
    let Some(snapshot) = ctx.server.data::<SecuritySnapshot>(security::ID) else {
        return;
    };
    for attacker in snapshot.attackers.iter().filter(|a| a.is_brute_force()) {
        let banned = if snapshot.is_banned(&attacker.ip) {
            "already banned"
        } else {
            "not banned"
        };
        out.push(Finding::new(
            Severity::Critical,
            "security",
            format!(
                "SSH brute force from {}: {} failures in 10 min, users {} ({banned})",
                attacker.ip,
                attacker.recent_failures,
                attacker.users_label()
            ),
        ));
    }
}

fn alerts(ctx: &DocContext<'_>, out: &mut Vec<Finding>) {
    for alert in ctx
        .state
        .active_alerts()
        .filter(|a| a.server == ctx.server.spec.id)
    {
        out.push(Finding::new(
            alert.severity,
            "alert",
            format!("{}: {}", alert.rule_name, alert.message),
        ));
    }
}

fn anomalies(ctx: &DocContext<'_>, out: &mut Vec<Finding>) {
    let Some(snapshot) = ctx.server.data::<AnomaliesSnapshot>(anomalies::ID) else {
        return;
    };
    for sign in &snapshot.signs {
        let peers = if sign.peers.is_empty() {
            String::new()
        } else {
            format!(" (peers: {})", sign.peers.join(", "))
        };
        out.push(Finding::new(
            sign.severity,
            "network",
            format!("{}: {}{peers}", sign.kind.label(), sign.detail),
        ));
    }
}
