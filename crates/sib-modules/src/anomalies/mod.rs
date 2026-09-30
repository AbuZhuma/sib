mod model;
mod parse;
mod signs;

use async_trait::async_trait;
use sib_core::{
    Availability, CollectContext, Event, Module, ModuleError, ModuleId, ModuleSettings, Sample,
    Schedule, Snapshot, Transport,
};

pub use model::{AnomaliesSnapshot, AttackSign, Counters, Peer, Rates, SignKind, TOP_PEERS};

use crate::common::rate::per_second;
use crate::common::{detect, sections};

pub const ID: ModuleId = ModuleId("anomalies");
pub const KEY_SYN_RECV: &str = "anomalies.syn_recv";
pub const KEY_PPS_IN: &str = "anomalies.pps_in";
pub const KEY_PPS_OUT: &str = "anomalies.pps_out";
pub const KEY_NEW_CONNECTIONS: &str = "anomalies.new_conn_per_s";
pub const KEY_TOP_SHARE: &str = "anomalies.top_share_pct";
pub const KEY_PEERS: &str = "anomalies.peers";
pub const KEY_ATTACK: &str = "anomalies.attack";

const SCRIPT_PARTS: [(&str, &str); 5] = [
    ("ss", "ss -Htan"),
    ("netstat", "cat /proc/net/netstat"),
    ("snmp", "cat /proc/net/snmp"),
    ("dev", "cat /proc/net/dev"),
    (
        "conntrack",
        "cat /proc/sys/net/netfilter/nf_conntrack_count /proc/sys/net/netfilter/nf_conntrack_max",
    ),
];

pub struct AnomaliesModule;

#[async_trait]
impl Module for AnomaliesModule {
    fn id(&self) -> ModuleId {
        ID
    }

    fn title(&self) -> &'static str {
        "Аномалии"
    }

    fn schedule(&self) -> Schedule {
        Schedule::Fast
    }

    async fn detect(
        &self,
        transport: &dyn Transport,
        _settings: &ModuleSettings,
    ) -> Result<Availability, ModuleError> {
        let probe = detect::require(
            transport,
            "command -v ss && test -r /proc/net/snmp",
            "нет ss или /proc/net/snmp",
        )
        .await?;
        if !probe.is_usable() {
            return Ok(probe);
        }
        Ok(Availability::Available)
    }

    async fn collect(
        &self,
        transport: &dyn Transport,
        context: &CollectContext,
    ) -> Result<Snapshot, ModuleError> {
        let output = transport.exec(&sections::script(&SCRIPT_PARTS)).await?;
        let mut snapshot = parse::anomalies_snapshot(&output.stdout)?;
        let previous = context.previous::<AnomaliesSnapshot>();
        snapshot.rates =
            previous.map(|(p, elapsed)| rates(&p.counters, &snapshot.counters, elapsed));
        snapshot.signs = signs::detect(&snapshot);
        let events = previous
            .map(|(p, _)| events_between(p, &snapshot))
            .unwrap_or_default();
        let samples = samples(&snapshot);
        Ok(Snapshot::new(snapshot)
            .with_samples(samples)
            .with_events(events))
    }
}

fn rates(previous: &Counters, current: &Counters, elapsed: f64) -> Rates {
    Rates {
        pps_in: per_second(current.rx_packets, previous.rx_packets, elapsed),
        pps_out: per_second(current.tx_packets, previous.tx_packets, elapsed),
        new_connections: per_second(current.passive_opens, previous.passive_opens, elapsed),
        failed_connections: per_second(current.attempt_fails, previous.attempt_fails, elapsed),
        syncookies: per_second(current.syncookies_sent, previous.syncookies_sent, elapsed),
        listen_drops: per_second(
            current.listen_drops + current.listen_overflows,
            previous.listen_drops + previous.listen_overflows,
            elapsed,
        ),
        udp_in: per_second(current.udp_in, previous.udp_in, elapsed),
    }
}

fn samples(snapshot: &AnomaliesSnapshot) -> Vec<Sample> {
    let mut samples = vec![
        Sample::new(KEY_SYN_RECV, snapshot.syn_recv() as f64),
        Sample::new(KEY_TOP_SHARE, snapshot.top_share_pct()),
        Sample::new(KEY_PEERS, snapshot.distinct_peers as f64),
        Sample::new(KEY_ATTACK, u8::from(snapshot.is_under_attack()) as f64),
    ];
    if let Some(rates) = &snapshot.rates {
        samples.push(Sample::new(KEY_PPS_IN, rates.pps_in));
        samples.push(Sample::new(KEY_PPS_OUT, rates.pps_out));
        samples.push(Sample::new(KEY_NEW_CONNECTIONS, rates.new_connections));
    }
    samples
}

fn events_between(previous: &AnomaliesSnapshot, current: &AnomaliesSnapshot) -> Vec<Event> {
    current
        .signs
        .iter()
        .filter(|sign| !previous.signs.iter().any(|p| p.kind == sign.kind))
        .map(|sign| {
            let peers = if sign.peers.is_empty() {
                String::new()
            } else {
                format!(" [{}]", sign.peers.join(", "))
            };
            Event::new(
                ID,
                sign.severity,
                format!("{}: {}{peers}", sign.kind.label(), sign.detail),
            )
        })
        .collect()
}
