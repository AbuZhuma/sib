use chrono::{DateTime, Utc};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Protocol {
    Tcp,
    Udp,
}

impl Protocol {
    pub fn label(self) -> &'static str {
        match self {
            Self::Tcp => "tcp",
            Self::Udp => "udp",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ListeningPort {
    pub protocol: Protocol,
    pub address: String,
    pub port: u16,
    pub pid: Option<u32>,
    pub process: Option<String>,
    pub connections: u64,
    pub reachable: Option<bool>,
    pub firewall_allowed: Option<bool>,
}

impl ListeningPort {
    pub fn is_wildcard(&self) -> bool {
        matches!(
            self.address.as_str(),
            "0.0.0.0" | "*" | "[::]" | "::" | "%lo:*"
        )
    }

    pub fn is_loopback(&self) -> bool {
        let bare = self.address.trim_matches(['[', ']']);
        let v4 = bare.strip_prefix("::ffff:").unwrap_or(bare);
        v4.starts_with("127.") || bare == "::1"
    }

    pub fn is_public(&self) -> bool {
        self.is_wildcard() || !self.is_loopback()
    }

    pub fn same_socket(&self, other: &Self) -> bool {
        self.protocol == other.protocol && self.port == other.port && self.address == other.address
    }

    pub fn process_label(&self) -> String {
        match (&self.process, self.pid) {
            (Some(name), Some(pid)) => format!("{name}[{pid}]"),
            (Some(name), None) => name.clone(),
            (None, _) => "?".to_owned(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FirewallBackend {
    Ufw,
    Firewalld,
    Nftables,
    Iptables,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Firewall {
    pub backend: FirewallBackend,
    pub active: bool,
    pub allowed: Vec<(Protocol, u16)>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PortsSnapshot {
    pub ports: Vec<ListeningPort>,
    pub firewall: Option<Firewall>,
    pub checked_at: Option<DateTime<Utc>>,
}

impl PortsSnapshot {
    pub fn public_ports(&self) -> impl Iterator<Item = &ListeningPort> {
        self.ports
            .iter()
            .filter(|p| p.is_wildcard() || !p.is_loopback())
    }

    pub fn exposed_without_firewall(&self) -> impl Iterator<Item = &ListeningPort> {
        self.public_ports().filter(|p| {
            p.protocol == Protocol::Tcp
                && p.reachable == Some(true)
                && p.firewall_allowed != Some(true)
        })
    }
}

pub fn apply_firewall(snapshot: &mut PortsSnapshot) {
    let Some(firewall) = &snapshot.firewall else {
        return;
    };
    if !firewall.active {
        return;
    }
    for port in &mut snapshot.ports {
        port.firewall_allowed = Some(firewall.allowed.contains(&(port.protocol, port.port)));
    }
}
