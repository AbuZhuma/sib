mod checks;
mod events;
mod journal;
mod model;
mod parse;

use asiba_core::{
    Availability, CollectContext, Module, ModuleError, ModuleId, Sample, Schedule, Snapshot,
    SudoMode, Transport,
};
use async_trait::async_trait;
use chrono::Utc;

pub use checks::{Check, CheckStatus, checks, score};
pub use model::{
    Attacker, BRUTE_FORCE_THRESHOLD, BRUTE_FORCE_WINDOW_MINUTES, Ban, BanBackend, FirewallState,
    Jail, Login, SecuritySnapshot, SshdSettings, SudoCall, Switch,
};

use crate::common::sections;

pub const ID: ModuleId = ModuleId("security");
pub const KEY_FAILED_LOGINS: &str = "security.failed_logins";
pub const KEY_ATTACKERS: &str = "security.attackers";
pub const KEY_BANS: &str = "security.bans";

const SSH_LOG_LINES: u32 = 2000;
const SUDO_LOG_LINES: u32 = 500;
const LOG_SINCE: &str = "-24h";

fn script() -> String {
    let ssh = format!(
        "if command -v journalctl >/dev/null; then journalctl -q --no-pager -o short-iso --since {LOG_SINCE} -t sshd -t sshd-session -n {SSH_LOG_LINES}; else grep -h 'sshd' /var/log/auth.log /var/log/secure | tail -n {SSH_LOG_LINES}; fi"
    );
    let sudo = format!(
        "if command -v journalctl >/dev/null; then journalctl -q --no-pager -o short-iso --since {LOG_SINCE} -t sudo -n {SUDO_LOG_LINES}; else grep -h 'sudo' /var/log/auth.log /var/log/secure | tail -n {SUDO_LOG_LINES}; fi"
    );
    let parts = [
        ("whoami", "id -un"),
        (
            "units",
            "for s in firewalld ufw nftables iptables netfilter-persistent fail2ban; do printf '%s %s\\n' \"$s\" \"$(systemctl is-active \"$s\" 2>/dev/null)\"; done",
        ),
        (
            "tools",
            "for t in fail2ban-client nft iptables ufw; do command -v \"$t\" >/dev/null && echo \"$t\"; done",
        ),
        ("ssh", ssh.as_str()),
        ("sudo", sudo.as_str()),
        (
            "fail2ban",
            "fail2ban-client status | sed -n 's/.*Jail list:[[:space:]]*//p' | tr ',' '\\n' | while read -r j; do [ -n \"$j\" ] && echo \"@@ $j\" && fail2ban-client status \"$j\"; done",
        ),
        (
            "sshd",
            "sshd -T 2>/dev/null | grep -iE '^(passwordauthentication|permitrootlogin|pubkeyauthentication|port|maxauthtries) ' || grep -hiE '^[[:space:]]*(PasswordAuthentication|PermitRootLogin|PubkeyAuthentication|Port|MaxAuthTries)[[:space:]]' /etc/ssh/sshd_config /etc/ssh/sshd_config.d/*.conf",
        ),
        (
            "hashes",
            "sha256sum /etc/passwd /etc/group /etc/sudoers /etc/sudoers.d/*",
        ),
        (
            "nftbans",
            "nft list set inet asiba bans; nft list set inet asiba bans6",
        ),
        ("iptbans", "iptables -S ASIBA; ip6tables -S ASIBA"),
        ("ufwbans", "ufw status | grep DENY"),
    ];
    sections::script(&parts)
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

    async fn detect(&self, transport: &dyn Transport) -> Result<Availability, ModuleError> {
        if transport.sudo_mode() == SudoMode::None {
            let whoami = transport.exec("id -un").await?;
            if whoami.stdout.trim() != "root" {
                return Ok(Availability::Partial {
                    missing: vec![
                        "настройки sshd, fail2ban и список банов (нужен sudo)".to_owned(),
                    ],
                });
            }
        }
        Ok(Availability::Available)
    }

    async fn collect(
        &self,
        transport: &dyn Transport,
        context: &CollectContext,
    ) -> Result<Snapshot, ModuleError> {
        let command = script();
        let output = match transport.sudo_mode() {
            SudoMode::None => transport.exec(&command).await?,
            _ => match transport.exec_root(&command).await {
                Ok(output) if output.is_success() => output,
                _ => transport.exec(&command).await?,
            },
        };
        let now = Utc::now();
        let snapshot = parse::security_snapshot(&output.stdout, now)?;
        let events = context
            .previous::<SecuritySnapshot>()
            .map(|(previous, _)| events::between(previous, &snapshot, context.previous_taken_at()))
            .unwrap_or_default();
        let samples = vec![
            Sample::new(KEY_FAILED_LOGINS, snapshot.failed_logins as f64),
            Sample::new(KEY_ATTACKERS, snapshot.attackers.len() as f64),
            Sample::new(KEY_BANS, snapshot.bans.len() as f64),
        ];
        Ok(Snapshot::new(snapshot)
            .with_samples(samples)
            .with_events(events))
    }
}
