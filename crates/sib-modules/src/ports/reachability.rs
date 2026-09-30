use std::time::Duration;

use chrono::Utc;
use tokio::net::TcpStream;
use tokio::task::JoinSet;
use tokio::time::timeout;

use super::model::{PortsSnapshot, Protocol};

const CONNECT_TIMEOUT: Duration = Duration::from_millis(1500);
const RECHECK_AFTER: chrono::Duration = chrono::Duration::minutes(5);
const MAX_PORTS: usize = 64;

pub async fn fill(snapshot: &mut PortsSnapshot, previous: Option<&PortsSnapshot>, host: &str) {
    if host.is_empty() {
        return;
    }
    if let Some(previous) = previous.filter(|p| is_fresh(p, snapshot)) {
        reuse(snapshot, previous);
        return;
    }
    let targets: Vec<u16> = snapshot
        .public_ports()
        .filter(|p| p.protocol == Protocol::Tcp)
        .map(|p| p.port)
        .take(MAX_PORTS)
        .collect();
    let mut checks = JoinSet::new();
    for port in targets {
        let host = host.to_owned();
        checks.spawn(async move { (port, connect(&host, port).await) });
    }
    while let Some(Ok((port, reachable))) = checks.join_next().await {
        for entry in snapshot
            .ports
            .iter_mut()
            .filter(|p| p.port == port && p.protocol == Protocol::Tcp)
        {
            entry.reachable = Some(reachable);
        }
    }
    snapshot.checked_at = Some(Utc::now());
}

fn is_fresh(previous: &PortsSnapshot, current: &PortsSnapshot) -> bool {
    let Some(checked_at) = previous.checked_at else {
        return false;
    };
    let same_sockets = previous.ports.len() == current.ports.len()
        && previous
            .ports
            .iter()
            .zip(&current.ports)
            .all(|(a, b)| a.same_socket(b));
    same_sockets && Utc::now() - checked_at < RECHECK_AFTER
}

fn reuse(snapshot: &mut PortsSnapshot, previous: &PortsSnapshot) {
    for (current, before) in snapshot.ports.iter_mut().zip(&previous.ports) {
        current.reachable = before.reachable;
    }
    snapshot.checked_at = previous.checked_at;
}

async fn connect(host: &str, port: u16) -> bool {
    matches!(
        timeout(CONNECT_TIMEOUT, TcpStream::connect((host, port))).await,
        Ok(Ok(_))
    )
}
