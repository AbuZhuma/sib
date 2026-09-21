use std::collections::BTreeMap;

use chrono::{DateTime, Duration, Utc};

pub const BRUTE_FORCE_WINDOW_MINUTES: i64 = 10;
pub const BRUTE_FORCE_THRESHOLD: u64 = 10;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Attacker {
    pub ip: String,
    pub failures: u64,
    pub recent_failures: u64,
    pub users: Vec<String>,
    pub first_at: DateTime<Utc>,
    pub last_at: DateTime<Utc>,
}

impl Attacker {
    pub fn is_brute_force(&self) -> bool {
        self.recent_failures >= BRUTE_FORCE_THRESHOLD
    }

    pub fn users_label(&self) -> String {
        const SHOWN: usize = 4;
        let mut label = self
            .users
            .iter()
            .take(SHOWN)
            .cloned()
            .collect::<Vec<_>>()
            .join(", ");
        if self.users.len() > SHOWN {
            label.push_str(&format!(" +{}", self.users.len() - SHOWN));
        }
        label
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Login {
    pub at: DateTime<Utc>,
    pub user: String,
    pub from: String,
    pub method: String,
}

impl Login {
    pub fn same(&self, other: &Self) -> bool {
        self.at == other.at && self.user == other.user && self.from == other.from
    }

    pub fn is_root(&self) -> bool {
        self.user == "root"
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SudoCall {
    pub at: DateTime<Utc>,
    pub user: String,
    pub target_user: String,
    pub command: String,
    pub is_success: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Jail {
    pub name: String,
    pub currently_failed: u64,
    pub total_banned: u64,
    pub banned: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Switch {
    On,
    Off,
    Unknown,
}

impl Switch {
    pub fn from_yes_no(value: &str) -> Self {
        match value.trim().to_ascii_lowercase().as_str() {
            "yes" => Self::On,
            "no" => Self::Off,
            _ => Self::Unknown,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct SshdSettings {
    pub password_auth: Option<Switch>,
    pub permit_root_login: Option<String>,
    pub pubkey_auth: Option<Switch>,
    pub port: Option<u16>,
    pub max_auth_tries: Option<u32>,
    pub permit_empty_passwords: Option<Switch>,
    pub x11_forwarding: Option<Switch>,
    pub login_grace_time: Option<u32>,
    pub client_alive_interval: Option<u32>,
    pub allow_tcp_forwarding: Option<Switch>,
    pub use_pam: Option<Switch>,
    pub max_startups: Option<String>,
}

impl SshdSettings {
    pub fn is_known(&self) -> bool {
        self.password_auth.is_some() || self.permit_root_login.is_some()
    }

    pub fn root_login_allowed(&self) -> Option<bool> {
        let value = self.permit_root_login.as_deref()?;
        Some(value == "yes")
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FirewallState {
    Active(String),
    Inactive,
    Unknown,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BanBackend {
    Fail2ban,
    Nftables,
    Iptables,
    Ufw,
}

impl BanBackend {
    pub fn label(self) -> &'static str {
        match self {
            Self::Fail2ban => "fail2ban",
            Self::Nftables => "nftables",
            Self::Iptables => "iptables",
            Self::Ufw => "ufw",
        }
    }

    pub fn supports_timeout(self) -> bool {
        matches!(self, Self::Nftables)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Ban {
    pub ip: String,
    pub source: String,
    pub expires: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MacStatus {
    SelinuxEnforcing,
    SelinuxPermissive,
    AppArmor,
    Disabled,
    Unknown,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Hardening {
    pub sysctl: BTreeMap<String, String>,
    pub mac: Option<MacStatus>,
    pub ntp_synced: Option<bool>,
    pub extra_uid0: Vec<String>,
    pub empty_passwords: Option<Vec<String>>,
    pub sudo_nopasswd: Option<u32>,
    pub writable_keys: Option<Vec<String>>,
    pub world_writable_etc: Vec<String>,
    pub risky_ports: Vec<u16>,
    pub auto_updates: bool,
    pub auditd: bool,
}

impl Hardening {
    pub fn sysctl(&self, key: &str) -> Option<&str> {
        self.sysctl.get(key).map(String::as_str)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SecuritySnapshot {
    pub attackers: Vec<Attacker>,
    pub failed_logins: u64,
    pub logins: Vec<Login>,
    pub sudo_calls: Vec<SudoCall>,
    pub jails: Vec<Jail>,
    pub has_fail2ban: bool,
    pub sshd: SshdSettings,
    pub firewall: FirewallState,
    pub file_hashes: Vec<(String, String)>,
    pub bans: Vec<Ban>,
    pub ban_backend: Option<BanBackend>,
    pub is_root_view: bool,
    pub hardening: Hardening,
}

impl SecuritySnapshot {
    pub fn brute_force_count(&self) -> usize {
        self.attackers.iter().filter(|a| a.is_brute_force()).count()
    }

    pub fn is_banned(&self, ip: &str) -> bool {
        self.bans.iter().any(|b| b.ip == ip)
    }

    pub fn logins_since(&self, at: DateTime<Utc>) -> impl Iterator<Item = &Login> {
        self.logins.iter().filter(move |l| l.at > at)
    }

    pub fn has_login_from(&self, from: &str, before: DateTime<Utc>) -> bool {
        self.logins.iter().any(|l| l.from == from && l.at <= before)
    }

    pub fn hash_of(&self, path: &str) -> Option<&str> {
        self.file_hashes
            .iter()
            .find(|(p, _)| p == path)
            .map(|(_, h)| h.as_str())
    }
}

pub fn brute_force_since(now: DateTime<Utc>) -> DateTime<Utc> {
    now - Duration::minutes(BRUTE_FORCE_WINDOW_MINUTES)
}
