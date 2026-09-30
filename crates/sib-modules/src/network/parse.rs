use std::collections::BTreeMap;

use sib_core::ModuleError;

use super::model::{Counters, Interface, NetworkSnapshot};
use crate::common::sections::Sections;

pub fn network_snapshot(raw: &str) -> Result<NetworkSnapshot, ModuleError> {
    let sections = Sections::parse(raw);
    let mut interfaces = interfaces(sections.get_or_empty("dev"))?;
    apply_addresses(&mut interfaces, sections.get_or_empty("addr"));
    apply_links(&mut interfaces, sections.get_or_empty("link"));
    apply_sys(&mut interfaces, sections.get_or_empty("sys"));
    let (gateway, default_interface) = route(sections.get_or_empty("route"));
    Ok(NetworkSnapshot {
        interfaces,
        connections: connections(sections.get_or_empty("conn")),
        gateway,
        default_interface,
    })
}

fn interfaces(raw: &str) -> Result<Vec<Interface>, ModuleError> {
    let parsed: Vec<Interface> = raw.lines().skip(2).filter_map(dev_line).collect();
    if parsed.is_empty() {
        return Err(ModuleError::Parse("empty /proc/net/dev".to_owned()));
    }
    Ok(parsed)
}

fn dev_line(line: &str) -> Option<Interface> {
    let (name, rest) = line.split_once(':')?;
    let values: Vec<u64> = rest
        .split_whitespace()
        .filter_map(|v| v.parse().ok())
        .collect();
    if values.len() < 16 {
        return None;
    }
    Some(Interface {
        name: name.trim().to_owned(),
        mac: None,
        state: "unknown".to_owned(),
        speed_mbps: None,
        mtu: None,
        addresses: Vec::new(),
        rx: Counters {
            bytes: values[0],
            packets: values[1],
            errors: values[2],
            drops: values[3],
        },
        tx: Counters {
            bytes: values[8],
            packets: values[9],
            errors: values[10],
            drops: values[11],
        },
        rates: None,
    })
}

fn apply_addresses(interfaces: &mut [Interface], raw: &str) {
    for line in raw.lines() {
        let fields: Vec<&str> = line.split_whitespace().collect();
        if fields.len() < 4 || fields[2] != "inet" && fields[2] != "inet6" {
            continue;
        }
        if fields.contains(&"scope") && line.contains("scope link") {
            continue;
        }
        if let Some(interface) = interfaces.iter_mut().find(|i| i.name == fields[1]) {
            interface.addresses.push(fields[3].to_owned());
        }
    }
}

fn apply_links(interfaces: &mut [Interface], raw: &str) {
    for line in raw.lines() {
        let fields: Vec<&str> = line.split_whitespace().collect();
        let Some(name) = fields.get(1).map(|n| n.trim_end_matches(':')) else {
            continue;
        };
        let Some(interface) = interfaces.iter_mut().find(|i| i.name == name) else {
            continue;
        };
        interface.mtu = field_after(&fields, "mtu").and_then(|v| v.parse().ok());
        interface.mac = field_after(&fields, "link/ether").map(str::to_owned);
    }
}

fn apply_sys(interfaces: &mut [Interface], raw: &str) {
    for line in raw.lines() {
        let fields: Vec<&str> = line.split_whitespace().collect();
        let Some(interface) = fields
            .first()
            .and_then(|n| interfaces.iter_mut().find(|i| i.name == *n))
        else {
            continue;
        };
        if let Some(state) = fields.get(1) {
            interface.state = (*state).to_owned();
        }
        interface.speed_mbps = fields
            .get(2)
            .and_then(|v| v.parse::<i64>().ok())
            .filter(|v| *v > 0)
            .map(|v| v as u64);
    }
}

fn field_after<'a>(fields: &[&'a str], key: &str) -> Option<&'a str> {
    let index = fields.iter().position(|f| *f == key)?;
    fields.get(index + 1).copied()
}

fn connections(raw: &str) -> BTreeMap<String, u64> {
    raw.lines()
        .filter_map(|line| {
            let (count, state) = line.trim().split_once(' ')?;
            Some((state.trim().to_owned(), count.parse().ok()?))
        })
        .collect()
}

fn route(raw: &str) -> (Option<String>, Option<String>) {
    let fields: Vec<&str> = raw
        .lines()
        .next()
        .unwrap_or_default()
        .split_whitespace()
        .collect();
    let gateway = field_after(&fields, "via").map(str::to_owned);
    let device = field_after(&fields, "dev").map(str::to_owned);
    (gateway, device)
}

#[cfg(test)]
mod tests {
    use super::*;

    const LAPTOP: &str = include_str!("../../fixtures/network/laptop.txt");
    const VPS: &str = include_str!("../../fixtures/network/vps.txt");

    #[test]
    fn laptop_fixture_parses_interfaces_with_addresses() {
        let snapshot = network_snapshot(LAPTOP).expect("parse");
        let wifi = snapshot
            .interfaces
            .iter()
            .find(|i| i.name == "wlp0s20f3")
            .expect("wifi");
        assert_eq!(wifi.rx.bytes, 420976751);
        assert_eq!(wifi.rx.drops, 859);
        assert_eq!(wifi.addresses, vec!["192.168.0.196/24"]);
        assert_eq!(wifi.mtu, Some(1500));
        assert_eq!(wifi.state, "up");
        assert_eq!(snapshot.gateway.as_deref(), Some("192.168.0.1"));
        assert_eq!(snapshot.connections.get("ESTAB"), Some(&64));
    }

    #[test]
    fn vps_fixture_reads_speed_and_mac() {
        let snapshot = network_snapshot(VPS).expect("parse");
        let eth = snapshot
            .interfaces
            .iter()
            .find(|i| i.name == "eth0")
            .expect("eth0");
        assert_eq!(eth.speed_mbps, Some(1000));
        assert_eq!(eth.mac.as_deref(), Some("52:54:00:12:34:56"));
        assert_eq!(snapshot.default_interface.as_deref(), Some("eth0"));
        assert_eq!(snapshot.connections_total(), 23);
    }

    #[test]
    fn empty_dev_is_parse_error() {
        assert!(matches!(
            network_snapshot("###dev\n"),
            Err(ModuleError::Parse(_))
        ));
    }
}
