use sib_core::{AppState, IncidentDraft, IncidentKind, ServerState, Severity};
use sib_modules::security::{self, SecuritySnapshot};

use super::Detector;

pub struct BruteForceDetector;

impl Detector for BruteForceDetector {
    fn detect(&self, server: &ServerState, state: &AppState) -> Vec<IncidentDraft> {
        let Some(snapshot) = server.data::<SecuritySnapshot>(security::ID) else {
            return Vec::new();
        };
        snapshot
            .attackers
            .iter()
            .filter(|a| a.is_brute_force())
            .map(|attacker| {
                let banned = if snapshot.is_banned(&attacker.ip) {
                    "already banned"
                } else {
                    "not banned"
                };
                let country = state.country_of(&attacker.ip).unwrap_or("?");
                IncidentDraft::new(
                    IncidentKind::BruteForce,
                    Severity::Critical,
                    attacker.ip.clone(),
                    format!(
                        "SSH password guessing from {} ({country}): {} failed attempts in 10 minutes, users {} ({banned})",
                        attacker.ip,
                        attacker.recent_failures,
                        attacker.users_label()
                    ),
                )
                .evidence([format!("failures_24h: {}", attacker.failures)])
            })
            .collect()
    }
}
