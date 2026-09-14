use std::collections::BTreeMap;

use crate::common::rate;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Counters {
    pub bytes: u64,
    pub packets: u64,
    pub errors: u64,
    pub drops: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct InterfaceRates {
    pub rx_bps: f64,
    pub tx_bps: f64,
    pub rx_pps: f64,
    pub tx_pps: f64,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Interface {
    pub name: String,
    pub mac: Option<String>,
    pub state: String,
    pub speed_mbps: Option<u64>,
    pub mtu: Option<u32>,
    pub addresses: Vec<String>,
    pub rx: Counters,
    pub tx: Counters,
    pub rates: Option<InterfaceRates>,
}

impl Interface {
    pub fn is_loopback(&self) -> bool {
        self.name == "lo"
    }

    pub fn is_up(&self) -> bool {
        self.state == "up" || self.state == "unknown"
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct NetworkSnapshot {
    pub interfaces: Vec<Interface>,
    pub connections: BTreeMap<String, u64>,
    pub gateway: Option<String>,
    pub default_interface: Option<String>,
}

impl NetworkSnapshot {
    pub fn primary_addresses(&self) -> Vec<&str> {
        self.interfaces
            .iter()
            .filter(|i| !i.is_loopback())
            .flat_map(|i| i.addresses.iter().map(String::as_str))
            .collect()
    }

    pub fn connections_total(&self) -> u64 {
        self.connections.values().sum()
    }
}

pub fn fill_rates(current: &mut NetworkSnapshot, previous: &NetworkSnapshot, elapsed: f64) {
    for interface in &mut current.interfaces {
        let Some(before) = previous
            .interfaces
            .iter()
            .find(|i| i.name == interface.name)
        else {
            continue;
        };
        interface.rates = Some(InterfaceRates {
            rx_bps: rate::per_second(interface.rx.bytes, before.rx.bytes, elapsed),
            tx_bps: rate::per_second(interface.tx.bytes, before.tx.bytes, elapsed),
            rx_pps: rate::per_second(interface.rx.packets, before.rx.packets, elapsed),
            tx_pps: rate::per_second(interface.tx.packets, before.tx.packets, elapsed),
        });
    }
}
