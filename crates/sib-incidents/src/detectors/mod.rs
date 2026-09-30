mod alerts;
mod anomalies;
mod brute_force;
mod containers;
mod deploys;
mod disks;
mod memory;
mod modules;
mod security_checks;
mod system_clock;
mod units;
mod updates;

use sib_core::{AppState, IncidentDraft, ServerState};

pub trait Detector: Send + Sync {
    fn detect(&self, server: &ServerState, state: &AppState) -> Vec<IncidentDraft>;
}

fn all() -> Vec<Box<dyn Detector>> {
    vec![
        Box::new(alerts::AlertsDetector),
        Box::new(anomalies::AnomaliesDetector),
        Box::new(brute_force::BruteForceDetector),
        Box::new(security_checks::SecurityChecksDetector),
        Box::new(units::UnitsDetector),
        Box::new(containers::ContainersDetector),
        Box::new(deploys::DeploysDetector),
        Box::new(disks::DisksDetector),
        Box::new(memory::MemoryDetector),
        Box::new(updates::UpdatesDetector),
        Box::new(modules::ModulesDetector),
        Box::new(system_clock::ClockDetector),
    ]
}

pub fn detect(server: &ServerState, state: &AppState) -> Vec<IncidentDraft> {
    let mut drafts: Vec<IncidentDraft> = all()
        .iter()
        .flat_map(|detector| detector.detect(server, state))
        .collect();
    drafts.sort_by_key(|d| std::cmp::Reverse(d.severity));
    drafts
}
