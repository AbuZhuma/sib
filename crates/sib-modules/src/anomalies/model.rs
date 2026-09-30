use std::collections::BTreeMap;

use sib_core::Severity;

pub const TOP_PEERS: usize = 10;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Peer {
    pub ip: String,
    pub connections: u64,
    pub syn_recv: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Counters {
    pub rx_packets: u64,
    pub tx_packets: u64,
    pub active_opens: u64,
    pub passive_opens: u64,
    pub attempt_fails: u64,
    pub syncookies_sent: u64,
    pub listen_drops: u64,
    pub listen_overflows: u64,
    pub udp_in: u64,
    pub udp_errors: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Rates {
    pub pps_in: f64,
    pub pps_out: f64,
    pub new_connections: f64,
    pub failed_connections: f64,
    pub syncookies: f64,
    pub listen_drops: f64,
    pub udp_in: f64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SignKind {
    SynFlood,
    SynBacklog,
    ListenDrops,
    PacketFlood,
    ConnectionFlood,
    SinglePeer,
    ConcentratedPeers,
    ConntrackFull,
}

impl SignKind {
    pub fn label(self) -> &'static str {
        match self {
            Self::SynFlood => "SYN flood",
            Self::SynBacklog => "SYN backlog is full",
            Self::ListenDrops => "the kernel is dropping incoming connections",
            Self::PacketFlood => "packet storm",
            Self::ConnectionFlood => "burst of new connections",
            Self::SinglePeer => "too many connections from one IP",
            Self::ConcentratedPeers => "traffic comes from a few IPs",
            Self::ConntrackFull => "the conntrack table is nearly full",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AttackSign {
    pub kind: SignKind,
    pub severity: Severity,
    pub detail: String,
    pub peers: Vec<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct AnomaliesSnapshot {
    pub states: BTreeMap<String, u64>,
    pub total_connections: u64,
    pub distinct_peers: u64,
    pub top_peers: Vec<Peer>,
    pub counters: Counters,
    pub rates: Option<Rates>,
    pub conntrack: Option<(u64, u64)>,
    pub signs: Vec<AttackSign>,
}

impl AnomaliesSnapshot {
    pub fn state(&self, name: &str) -> u64 {
        self.states.get(name).copied().unwrap_or(0)
    }

    pub fn syn_recv(&self) -> u64 {
        self.state("SYN-RECV")
    }

    pub fn established(&self) -> u64 {
        self.state("ESTAB")
    }

    pub fn top_share_pct(&self) -> f64 {
        if self.total_connections == 0 {
            return 0.0;
        }
        let top: u64 = self.top_peers.iter().map(|p| p.connections).sum();
        top as f64 * 100.0 / self.total_connections as f64
    }

    pub fn is_under_attack(&self) -> bool {
        self.signs.iter().any(|s| s.severity == Severity::Critical)
    }

    pub fn has_signs(&self) -> bool {
        !self.signs.is_empty()
    }

    pub fn suspicious_peers(&self) -> Vec<&str> {
        let mut peers: Vec<&str> = self
            .signs
            .iter()
            .flat_map(|s| s.peers.iter().map(String::as_str))
            .collect();
        peers.sort_unstable();
        peers.dedup();
        peers
    }
}
