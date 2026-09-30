use std::collections::BTreeMap;

use sib_core::ModuleError;

use super::model::{AnomaliesSnapshot, Counters, Peer, TOP_PEERS};
use crate::common::sections::Sections;

const SKIP_STATES: [&str; 1] = ["LISTEN"];

pub fn anomalies_snapshot(raw: &str) -> Result<AnomaliesSnapshot, ModuleError> {
    let sections = Sections::parse(raw);
    let sockets = sections.get_or_empty("ss");
    if sections.get("snmp").is_none() {
        return Err(ModuleError::Parse("нет /proc/net/snmp".to_owned()));
    }
    let (states, peers) = sockets_summary(sockets);
    let total_connections = states.values().sum();
    let mut top: Vec<Peer> = peers.into_values().collect();
    top.sort_by_key(|p| std::cmp::Reverse(p.connections));
    let distinct_peers = top.len() as u64;
    top.truncate(TOP_PEERS);
    Ok(AnomaliesSnapshot {
        states,
        total_connections,
        distinct_peers,
        top_peers: top,
        counters: counters(&sections),
        rates: None,
        conntrack: conntrack(sections.get_or_empty("conntrack")),
        signs: Vec::new(),
    })
}

fn sockets_summary(raw: &str) -> (BTreeMap<String, u64>, BTreeMap<String, Peer>) {
    let mut states = BTreeMap::new();
    let mut peers: BTreeMap<String, Peer> = BTreeMap::new();
    for line in raw.lines() {
        let fields: Vec<&str> = line.split_whitespace().collect();
        if fields.len() < 5 || SKIP_STATES.contains(&fields[0]) {
            continue;
        }
        let state = fields[0];
        *states.entry(state.to_owned()).or_insert(0) += 1;
        let Some(ip) = peer_ip(fields[4]) else {
            continue;
        };
        let peer = peers.entry(ip.clone()).or_insert_with(|| Peer {
            ip,
            connections: 0,
            syn_recv: 0,
        });
        peer.connections += 1;
        if state == "SYN-RECV" {
            peer.syn_recv += 1;
        }
    }
    (states, peers)
}

pub fn peer_ip(address: &str) -> Option<String> {
    let host = if let Some(rest) = address.strip_prefix('[') {
        rest.split(']').next()?
    } else {
        address.rsplit_once(':')?.0
    };
    let host = host.strip_prefix("::ffff:").unwrap_or(host);
    let host = host.split('%').next().unwrap_or(host);
    if host.is_empty() || host == "*" || host.starts_with("127.") || host == "::1" {
        return None;
    }
    Some(host.to_owned())
}

fn counters(sections: &Sections<'_>) -> Counters {
    let netstat = table(sections.get_or_empty("netstat"), "TcpExt:");
    let tcp = table(sections.get_or_empty("snmp"), "Tcp:");
    let udp = table(sections.get_or_empty("snmp"), "Udp:");
    let (rx_packets, tx_packets) = dev_packets(sections.get_or_empty("dev"));
    let get = |map: &BTreeMap<String, u64>, key: &str| map.get(key).copied().unwrap_or(0);
    Counters {
        rx_packets,
        tx_packets,
        active_opens: get(&tcp, "ActiveOpens"),
        passive_opens: get(&tcp, "PassiveOpens"),
        attempt_fails: get(&tcp, "AttemptFails"),
        syncookies_sent: get(&netstat, "SyncookiesSent"),
        listen_drops: get(&netstat, "ListenDrops"),
        listen_overflows: get(&netstat, "ListenOverflows"),
        udp_in: get(&udp, "InDatagrams"),
        udp_errors: get(&udp, "InErrors") + get(&udp, "RcvbufErrors"),
    }
}

fn table(raw: &str, prefix: &str) -> BTreeMap<String, u64> {
    let mut lines = raw.lines().filter(|l| l.starts_with(prefix));
    let (Some(header), Some(values)) = (lines.next(), lines.next()) else {
        return BTreeMap::new();
    };
    header
        .split_whitespace()
        .skip(1)
        .zip(values.split_whitespace().skip(1))
        .filter_map(|(key, value)| Some((key.to_owned(), value.parse::<i64>().ok()?.max(0) as u64)))
        .collect()
}

fn dev_packets(raw: &str) -> (u64, u64) {
    let mut rx = 0;
    let mut tx = 0;
    for line in raw.lines() {
        let Some((name, rest)) = line.split_once(':') else {
            continue;
        };
        if name.trim() == "lo" {
            continue;
        }
        let fields: Vec<u64> = rest
            .split_whitespace()
            .filter_map(|f| f.parse().ok())
            .collect();
        if fields.len() >= 10 {
            rx += fields[1];
            tx += fields[9];
        }
    }
    (rx, tx)
}

fn conntrack(raw: &str) -> Option<(u64, u64)> {
    let mut values = raw.lines().filter_map(|l| l.trim().parse::<u64>().ok());
    let count = values.next()?;
    let max = values.next()?;
    Some((count, max))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn attack_fixture_counts_states_and_top_peers() {
        let raw = include_str!("../../fixtures/anomalies/attack.txt");
        let snapshot = anomalies_snapshot(raw).expect("snapshot");
        assert_eq!(snapshot.syn_recv(), 6);
        assert_eq!(snapshot.established(), 4);
        assert_eq!(snapshot.total_connections, 11);
        assert_eq!(snapshot.top_peers[0].ip, "203.0.113.7");
        assert_eq!(snapshot.top_peers[0].connections, 6);
        assert_eq!(snapshot.top_peers[0].syn_recv, 5);
        assert_eq!(snapshot.distinct_peers, 4);
        assert_eq!(snapshot.counters.syncookies_sent, 1500);
        assert_eq!(snapshot.counters.listen_drops, 42);
        assert_eq!(snapshot.counters.passive_opens, 9686);
        assert_eq!(snapshot.counters.udp_in, 148617);
        assert_eq!(snapshot.counters.rx_packets, 1_000_000 + 5_000);
        assert_eq!(snapshot.conntrack, Some((260_000, 262_144)));
    }

    #[test]
    fn peer_ip_handles_v4_mapped_and_v6() {
        assert_eq!(peer_ip("[::ffff:1.2.3.4]:443").as_deref(), Some("1.2.3.4"));
        assert_eq!(peer_ip("[2001:db8::1]:22").as_deref(), Some("2001:db8::1"));
        assert_eq!(peer_ip("10.0.0.5:8080").as_deref(), Some("10.0.0.5"));
        assert_eq!(peer_ip("127.0.0.1:1"), None);
        assert_eq!(peer_ip("[::1]:1"), None);
    }

    #[test]
    fn missing_snmp_is_parse_error() {
        assert!(anomalies_snapshot("###ss\n").is_err());
    }
}
