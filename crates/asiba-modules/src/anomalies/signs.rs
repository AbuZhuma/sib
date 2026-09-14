use asiba_core::Severity;

use super::model::{AnomaliesSnapshot, AttackSign, Rates, SignKind};

const SYN_RECV_WARNING: u64 = 64;
const SYN_RECV_CRITICAL: u64 = 256;
const SYNCOOKIES_PER_SEC: f64 = 1.0;
const LISTEN_DROPS_PER_SEC: f64 = 5.0;
const PPS_WARNING: f64 = 50_000.0;
const PPS_CRITICAL: f64 = 200_000.0;
const NEW_CONNECTIONS_PER_SEC: f64 = 500.0;
const SINGLE_PEER_CONNECTIONS: u64 = 100;
const CONCENTRATION_MIN_CONNECTIONS: u64 = 200;
const CONCENTRATION_SHARE_PCT: f64 = 80.0;
const CONNTRACK_FULL_PCT: f64 = 90.0;

pub fn detect(snapshot: &AnomaliesSnapshot) -> Vec<AttackSign> {
    let mut signs = Vec::new();
    syn_backlog(snapshot, &mut signs);
    if let Some(rates) = &snapshot.rates {
        rate_signs(snapshot, rates, &mut signs);
    }
    peer_signs(snapshot, &mut signs);
    conntrack(snapshot, &mut signs);
    signs
}

fn syn_backlog(snapshot: &AnomaliesSnapshot, signs: &mut Vec<AttackSign>) {
    let syn_recv = snapshot.syn_recv();
    if syn_recv < SYN_RECV_WARNING {
        return;
    }
    let severity = if syn_recv >= SYN_RECV_CRITICAL {
        Severity::Critical
    } else {
        Severity::Warning
    };
    let peers = snapshot
        .top_peers
        .iter()
        .filter(|p| p.syn_recv > 0)
        .map(|p| p.ip.clone())
        .collect();
    signs.push(AttackSign {
        kind: SignKind::SynBacklog,
        severity,
        detail: format!("{syn_recv} полуоткрытых соединений"),
        peers,
    });
}

fn rate_signs(snapshot: &AnomaliesSnapshot, rates: &Rates, signs: &mut Vec<AttackSign>) {
    if rates.syncookies >= SYNCOOKIES_PER_SEC {
        signs.push(AttackSign {
            kind: SignKind::SynFlood,
            severity: Severity::Critical,
            detail: format!("ядро отправляет {:.0} SYN cookies/с", rates.syncookies),
            peers: syn_peers(snapshot),
        });
    }
    if rates.listen_drops >= LISTEN_DROPS_PER_SEC {
        signs.push(AttackSign {
            kind: SignKind::ListenDrops,
            severity: Severity::Critical,
            detail: format!("{:.0} сбросов/с в очереди accept", rates.listen_drops),
            peers: Vec::new(),
        });
    }
    if rates.pps_in >= PPS_WARNING {
        let severity = if rates.pps_in >= PPS_CRITICAL {
            Severity::Critical
        } else {
            Severity::Warning
        };
        signs.push(AttackSign {
            kind: SignKind::PacketFlood,
            severity,
            detail: format!("{:.0} пакетов/с входящих", rates.pps_in),
            peers: Vec::new(),
        });
    }
    if rates.new_connections >= NEW_CONNECTIONS_PER_SEC {
        signs.push(AttackSign {
            kind: SignKind::ConnectionFlood,
            severity: Severity::Warning,
            detail: format!("{:.0} новых соединений/с", rates.new_connections),
            peers: top_ips(snapshot, 3),
        });
    }
}

fn peer_signs(snapshot: &AnomaliesSnapshot, signs: &mut Vec<AttackSign>) {
    let heavy: Vec<String> = snapshot
        .top_peers
        .iter()
        .filter(|p| p.connections >= SINGLE_PEER_CONNECTIONS)
        .map(|p| p.ip.clone())
        .collect();
    if let Some(top) = snapshot.top_peers.first().filter(|_| !heavy.is_empty()) {
        signs.push(AttackSign {
            kind: SignKind::SinglePeer,
            severity: Severity::Warning,
            detail: format!("{}: {} соединений", top.ip, top.connections),
            peers: heavy,
        });
    }
    let share = snapshot.top_share_pct();
    if snapshot.total_connections >= CONCENTRATION_MIN_CONNECTIONS
        && share >= CONCENTRATION_SHARE_PCT
    {
        signs.push(AttackSign {
            kind: SignKind::ConcentratedPeers,
            severity: Severity::Warning,
            detail: format!(
                "топ-{} IP дают {share:.0}% соединений",
                snapshot.top_peers.len()
            ),
            peers: top_ips(snapshot, snapshot.top_peers.len()),
        });
    }
}

fn conntrack(snapshot: &AnomaliesSnapshot, signs: &mut Vec<AttackSign>) {
    let Some((count, max)) = snapshot.conntrack.filter(|(_, max)| *max > 0) else {
        return;
    };
    let pct = count as f64 * 100.0 / max as f64;
    if pct >= CONNTRACK_FULL_PCT {
        signs.push(AttackSign {
            kind: SignKind::ConntrackFull,
            severity: Severity::Critical,
            detail: format!("{count} из {max} ({pct:.0}%)"),
            peers: Vec::new(),
        });
    }
}

fn syn_peers(snapshot: &AnomaliesSnapshot) -> Vec<String> {
    snapshot
        .top_peers
        .iter()
        .filter(|p| p.syn_recv > 0)
        .map(|p| p.ip.clone())
        .collect()
}

fn top_ips(snapshot: &AnomaliesSnapshot, count: usize) -> Vec<String> {
    snapshot
        .top_peers
        .iter()
        .take(count)
        .map(|p| p.ip.clone())
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::anomalies::model::{Counters, Peer};
    use std::collections::BTreeMap;

    fn base() -> AnomaliesSnapshot {
        AnomaliesSnapshot {
            states: BTreeMap::new(),
            total_connections: 0,
            distinct_peers: 0,
            top_peers: Vec::new(),
            counters: Counters::default(),
            rates: None,
            conntrack: None,
            signs: Vec::new(),
        }
    }

    #[test]
    fn quiet_server_has_no_signs() {
        let mut snapshot = base();
        snapshot.states.insert("ESTAB".to_owned(), 40);
        snapshot.rates = Some(Rates::default());
        assert!(detect(&snapshot).is_empty());
    }

    #[test]
    fn syncookies_rate_is_critical_syn_flood_with_peers() {
        let mut snapshot = base();
        snapshot.rates = Some(Rates {
            syncookies: 120.0,
            ..Rates::default()
        });
        snapshot.top_peers.push(Peer {
            ip: "203.0.113.7".to_owned(),
            connections: 30,
            syn_recv: 30,
        });
        let signs = detect(&snapshot);
        assert_eq!(signs[0].kind, SignKind::SynFlood);
        assert_eq!(signs[0].severity, Severity::Critical);
        assert_eq!(signs[0].peers, vec!["203.0.113.7"]);
    }

    #[test]
    fn single_peer_over_threshold_is_warning() {
        let mut snapshot = base();
        snapshot.total_connections = 150;
        snapshot.top_peers.push(Peer {
            ip: "1.1.1.1".to_owned(),
            connections: 120,
            syn_recv: 0,
        });
        let signs = detect(&snapshot);
        assert_eq!(signs.len(), 1);
        assert_eq!(signs[0].kind, SignKind::SinglePeer);
    }

    #[test]
    fn conntrack_near_max_is_critical() {
        let mut snapshot = base();
        snapshot.conntrack = Some((250_000, 262_144));
        let signs = detect(&snapshot);
        assert_eq!(signs[0].kind, SignKind::ConntrackFull);
        assert!(snapshot_is_attack(&signs));
    }

    fn snapshot_is_attack(signs: &[AttackSign]) -> bool {
        signs.iter().any(|s| s.severity == Severity::Critical)
    }
}
