mod model;
mod parse;

use async_trait::async_trait;
use sib_core::{
    Availability, CollectContext, Module, ModuleError, ModuleId, ModuleSettings, Sample, Schedule,
    Snapshot, Transport,
};

pub use model::{Counters, Interface, InterfaceRates, NetworkSnapshot};

use crate::common::sections;

pub const ID: ModuleId = ModuleId("network");
pub const KEY_RX_BPS: &str = "network.rx_bps";
pub const KEY_TX_BPS: &str = "network.tx_bps";
pub const KEY_RX_PPS: &str = "network.rx_pps";
pub const KEY_TX_PPS: &str = "network.tx_pps";
pub const KEY_ESTABLISHED: &str = "network.conn.established";
pub const KEY_TIME_WAIT: &str = "network.conn.time_wait";
pub const KEY_SYN_RECV: &str = "network.conn.syn_recv";

const SCRIPT_PARTS: [(&str, &str); 6] = [
    ("dev", "cat /proc/net/dev"),
    ("addr", "ip -o addr"),
    ("link", "ip -o link"),
    (
        "sys",
        "for i in /sys/class/net/*; do printf '%s %s %s\\n' \"$(basename $i)\" \"$(cat $i/operstate)\" \"$(cat $i/speed)\"; done",
    ),
    ("conn", "ss -Htan | awk '{print $1}' | sort | uniq -c"),
    ("route", "ip -o -4 route show default"),
];

pub struct NetworkModule;

#[async_trait]
impl Module for NetworkModule {
    fn id(&self) -> ModuleId {
        ID
    }

    fn title(&self) -> &'static str {
        "Сеть"
    }

    fn schedule(&self) -> Schedule {
        Schedule::Fast
    }

    async fn detect(
        &self,
        _transport: &dyn Transport,
        _settings: &ModuleSettings,
    ) -> Result<Availability, ModuleError> {
        Ok(Availability::Available)
    }

    async fn collect(
        &self,
        transport: &dyn Transport,
        context: &CollectContext,
    ) -> Result<Snapshot, ModuleError> {
        let output = transport.exec(&sections::script(&SCRIPT_PARTS)).await?;
        let mut snapshot = parse::network_snapshot(&output.stdout)?;
        if let Some((previous, elapsed)) = context.previous::<NetworkSnapshot>() {
            model::fill_rates(&mut snapshot, previous, elapsed);
        }
        let samples = samples(&snapshot);
        Ok(Snapshot::new(snapshot).with_samples(samples))
    }
}

fn samples(snapshot: &NetworkSnapshot) -> Vec<Sample> {
    let mut samples = Vec::new();
    let mut total = InterfaceRates::default();
    let mut has_rates = false;
    for interface in snapshot.interfaces.iter().filter(|i| !i.is_loopback()) {
        let Some(rates) = &interface.rates else {
            continue;
        };
        has_rates = true;
        total.rx_bps += rates.rx_bps;
        total.tx_bps += rates.tx_bps;
        total.rx_pps += rates.rx_pps;
        total.tx_pps += rates.tx_pps;
        samples.push(Sample::new(
            interface_key(&interface.name, "rx_bps"),
            rates.rx_bps,
        ));
        samples.push(Sample::new(
            interface_key(&interface.name, "tx_bps"),
            rates.tx_bps,
        ));
    }
    if has_rates {
        samples.push(Sample::new(KEY_RX_BPS, total.rx_bps));
        samples.push(Sample::new(KEY_TX_BPS, total.tx_bps));
        samples.push(Sample::new(KEY_RX_PPS, total.rx_pps));
        samples.push(Sample::new(KEY_TX_PPS, total.tx_pps));
    }
    let count = |state: &str| snapshot.connections.get(state).copied().unwrap_or(0) as f64;
    samples.push(Sample::new(KEY_ESTABLISHED, count("ESTAB")));
    samples.push(Sample::new(KEY_TIME_WAIT, count("TIME-WAIT")));
    samples.push(Sample::new(KEY_SYN_RECV, count("SYN-RECV")));
    samples
}

pub fn interface_key(name: &str, metric: &str) -> String {
    format!("network.{name}.{metric}")
}
