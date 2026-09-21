mod actions;
mod bans;
mod events;
mod hardening;
mod journal;
mod model;
mod parse;
mod script;

use asiba_core::{
    ActionOutcome, ActionRequest, ActionSpec, Availability, CollectContext, Module, ModuleError,
    ModuleId, ModuleSettings, Sample, Schedule, Snapshot, SudoMode, Transport,
};
use async_trait::async_trait;
use chrono::{DateTime, Duration, Utc};

pub use actions::{ACTION_BAN, ACTION_UNBAN, PERMANENT, SPEC_BAN, SPEC_UNBAN};
pub use model::{
    Attacker, BRUTE_FORCE_THRESHOLD, BRUTE_FORCE_WINDOW_MINUTES, Ban, BanBackend, FirewallState,
    Hardening, Jail, Login, MacStatus, SecuritySnapshot, SshdSettings, SudoCall, Switch,
};

use crate::common::root::exec_prefer_root;

pub const ID: ModuleId = ModuleId("security");
pub const KEY_FAILED_LOGINS: &str = "security.failed_logins";
pub const KEY_ATTACKERS: &str = "security.attackers";
pub const KEY_BRUTE_FORCE: &str = "security.brute_force";
pub const KEY_BANS: &str = "security.bans";

pub const SLOW_PART_INTERVAL: Duration = Duration::minutes(5);

fn needs_slow_part(previous: Option<&SecuritySnapshot>, now: DateTime<Utc>) -> bool {
    previous.is_none_or(|p| now - p.slow_collected_at >= SLOW_PART_INTERVAL)
}

pub struct SecurityModule;

#[async_trait]
impl Module for SecurityModule {
    fn id(&self) -> ModuleId {
        ID
    }

    fn title(&self) -> &'static str {
        "Безопасность"
    }

    fn schedule(&self) -> Schedule {
        Schedule::Normal
    }

    async fn detect(
        &self,
        transport: &dyn Transport,
        _settings: &ModuleSettings,
    ) -> Result<Availability, ModuleError> {
        if transport.sudo_mode() == SudoMode::None {
            let whoami = transport.exec("id -un").await?;
            if whoami.stdout.trim() != "root" {
                return Ok(Availability::partial(
                    "настройки sshd, fail2ban и список банов (нужен sudo)",
                ));
            }
        }
        Ok(Availability::Available)
    }

    async fn collect(
        &self,
        transport: &dyn Transport,
        context: &CollectContext,
    ) -> Result<Snapshot, ModuleError> {
        let now = Utc::now();
        let previous = context.previous::<SecuritySnapshot>().map(|(p, _)| p);
        let include_slow = needs_slow_part(previous, now);
        let output = exec_prefer_root(transport, &script::build(include_slow)).await?;
        let mut snapshot = parse::security_snapshot(&output.stdout, now)?;
        if let Some(previous) = previous.filter(|_| !include_slow) {
            parse::carry_slow_part(&mut snapshot, previous);
        }
        let events = previous
            .map(|previous| events::between(previous, &snapshot, context.previous_taken_at()))
            .unwrap_or_default();
        let samples = vec![
            Sample::new(KEY_FAILED_LOGINS, snapshot.failed_logins as f64),
            Sample::new(KEY_ATTACKERS, snapshot.attackers.len() as f64),
            Sample::new(KEY_BRUTE_FORCE, snapshot.brute_force_count() as f64),
            Sample::new(KEY_BANS, snapshot.bans.len() as f64),
        ];
        Ok(Snapshot::new(snapshot)
            .with_samples(samples)
            .with_events(events))
    }

    fn actions(&self) -> &'static [ActionSpec] {
        &actions::SPECS
    }

    async fn perform(
        &self,
        transport: &dyn Transport,
        request: &ActionRequest,
    ) -> Result<ActionOutcome, ModuleError> {
        actions::perform(transport, request).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn script_without_slow_part_skips_hardening_commands() {
        let fast = script::build(false);
        assert!(fast.contains("###ssh"));
        assert!(fast.contains("###nftbans"));
        assert!(!fast.contains("###sysctl"));
        assert!(!fast.contains("###sshd"));
        let full = script::build(true);
        assert!(full.contains("###sysctl"));
        assert!(full.contains("###sshd"));
    }

    #[test]
    fn slow_part_is_due_without_previous_or_after_interval() {
        let now = Utc::now();
        assert!(needs_slow_part(None, now));
        let raw = include_str!("../../fixtures/security/server.txt");
        let mut previous = parse::security_snapshot(raw, now).expect("snapshot");
        previous.slow_collected_at = now;
        assert!(!needs_slow_part(
            Some(&previous),
            now + Duration::minutes(1)
        ));
        assert!(needs_slow_part(Some(&previous), now + SLOW_PART_INTERVAL));
    }
}
