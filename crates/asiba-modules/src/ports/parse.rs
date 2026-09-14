use std::collections::HashMap;

use asiba_core::ModuleError;

use super::model::{ListeningPort, PortsSnapshot, Protocol};
use crate::common::sections::Sections;

pub fn ports_snapshot(raw: &str) -> Result<PortsSnapshot, ModuleError> {
    let sections = Sections::parse(raw);
    let connections = connections_by_port(sections.get_or_empty("established"));
    let mut ports: Vec<ListeningPort> = sections
        .get_or_empty("listen")
        .lines()
        .filter_map(listen_line)
        .collect();
    if ports.is_empty() && !sections.get_or_empty("listen").trim().is_empty() {
        return Err(ModuleError::Parse(
            "не удалось разобрать вывод ss".to_owned(),
        ));
    }
    for port in &mut ports {
        port.connections = connections.get(&port.port).copied().unwrap_or(0);
    }
    ports.sort_by_key(|p| (p.port, p.protocol.label(), p.address.clone()));
    ports.dedup_by(|a, b| a.same_socket(b));
    Ok(PortsSnapshot {
        ports,
        firewall: None,
        checked_at: None,
    })
}

fn listen_line(line: &str) -> Option<ListeningPort> {
    let fields: Vec<&str> = line.split_whitespace().collect();
    if fields.len() < 5 {
        return None;
    }
    let protocol = match fields[0] {
        "tcp" => Protocol::Tcp,
        "udp" => Protocol::Udp,
        _ => return None,
    };
    let (address, port) = split_address(fields[4])?;
    let (process, pid) = fields.get(6).map(process_field).unwrap_or((None, None));
    Some(ListeningPort {
        protocol,
        address: address.to_owned(),
        port,
        pid,
        process,
        connections: 0,
        reachable: None,
        firewall_allowed: None,
    })
}

fn split_address(value: &str) -> Option<(&str, u16)> {
    let (address, port) = value.rsplit_once(':')?;
    let port = port.parse().ok()?;
    let address = address.split('%').next().unwrap_or(address);
    Some((address, port))
}

fn process_field(raw: &&str) -> (Option<String>, Option<u32>) {
    let inner = raw.strip_prefix("users:((").unwrap_or(raw);
    let name = inner.split('"').nth(1).map(str::to_owned);
    let pid = inner
        .split("pid=")
        .nth(1)
        .and_then(|rest| rest.split([',', ')']).next())
        .and_then(|v| v.parse().ok());
    (name, pid)
}

fn connections_by_port(raw: &str) -> HashMap<u16, u64> {
    let mut counts = HashMap::new();
    for line in raw.lines() {
        if let Some((_, port)) = split_address(line.trim()) {
            *counts.entry(port).or_insert(0) += 1;
        }
    }
    counts
}

#[cfg(test)]
mod tests {
    use super::*;

    const SERVER: &str = include_str!("../../fixtures/ports/server.txt");

    #[test]
    fn fixture_parses_listening_ports_with_processes() {
        let snapshot = ports_snapshot(SERVER).expect("parse");
        let ssh = snapshot
            .ports
            .iter()
            .find(|p| p.port == 22 && p.address == "0.0.0.0")
            .expect("ssh");
        assert_eq!(ssh.process.as_deref(), Some("sshd"));
        assert_eq!(ssh.pid, Some(812));
        assert_eq!(ssh.connections, 2);
        assert!(ssh.is_wildcard());
        let pg = snapshot
            .ports
            .iter()
            .find(|p| p.port == 5432)
            .expect("postgres");
        assert!(pg.is_loopback());
        assert_eq!(pg.process, None);
    }

    #[test]
    fn public_ports_exclude_loopback() {
        let snapshot = ports_snapshot(SERVER).expect("parse");
        let public: Vec<u16> = snapshot.public_ports().map(|p| p.port).collect();
        assert!(public.contains(&22));
        assert!(public.contains(&443));
        assert!(!public.contains(&5432));
    }

    #[test]
    fn mapped_ipv4_loopback_is_loopback() {
        let port = ListeningPort {
            protocol: Protocol::Tcp,
            address: "[::ffff:127.0.0.1]".into(),
            port: 1,
            pid: None,
            process: None,
            connections: 0,
            reachable: None,
            firewall_allowed: None,
        };
        assert!(port.is_loopback());
        assert!(!port.is_public());
    }

    #[test]
    fn empty_output_gives_no_ports() {
        let snapshot = ports_snapshot("###listen\n###established\n").expect("parse");
        assert!(snapshot.ports.is_empty());
    }
}
