mod firewall;
mod model;
mod parse;
mod reachability;

use asiba_core::{
    Availability, CollectContext, Event, Module, ModuleError, ModuleId, Sample, Schedule, Severity,
    Snapshot, SudoMode, Transport,
};
use async_trait::async_trait;

pub use model::{Firewall, FirewallBackend, ListeningPort, PortsSnapshot, Protocol};

use crate::common::{detect, sections};

pub const ID: ModuleId = ModuleId("ports");
pub const KEY_LISTENING: &str = "ports.listening";
pub const KEY_PUBLIC: &str = "ports.public";

const SCRIPT_PARTS: [(&str, &str); 2] = [
    ("listen", "ss -tulpnH"),
    (
        "established",
        "ss -Htan state established | awk '{print $3}'",
    ),
];
const FIREWALL_SCRIPT: &str = "if command -v ufw >/dev/null; then echo '###ufw'; ufw status; fi; \
if command -v firewall-cmd >/dev/null; then echo '###firewalld'; firewall-cmd --list-all; fi; \
if command -v nft >/dev/null; then echo '###nft'; nft list ruleset; fi; \
if command -v iptables >/dev/null; then echo '###iptables'; iptables -S INPUT; fi";

pub struct PortsModule;

#[async_trait]
impl Module for PortsModule {
    fn id(&self) -> ModuleId {
        ID
    }

    fn title(&self) -> &'static str {
        "Порты"
    }

    fn schedule(&self) -> Schedule {
        Schedule::Normal
    }

    async fn detect(&self, transport: &dyn Transport) -> Result<Availability, ModuleError> {
        let probe = detect::require(transport, "command -v ss", "нет ss (iproute2)").await?;
        if !probe.is_usable() {
            return Ok(probe);
        }
        if transport.sudo_mode() == SudoMode::None {
            return Ok(Availability::partial("файрвол (нужен sudo)"));
        }
        Ok(Availability::Available)
    }

    async fn collect(
        &self,
        transport: &dyn Transport,
        context: &CollectContext,
    ) -> Result<Snapshot, ModuleError> {
        let output = transport.exec(&sections::script(&SCRIPT_PARTS)).await?;
        let mut snapshot = parse::ports_snapshot(&output.stdout)?;
        snapshot.firewall = collect_firewall(transport).await;
        model::apply_firewall(&mut snapshot);
        let previous = context.previous::<PortsSnapshot>().map(|(p, _)| p);
        reachability::fill(&mut snapshot, previous, &context.host).await;
        let events = previous
            .map(|p| events_between(p, &snapshot))
            .unwrap_or_default();
        let samples = vec![
            Sample::new(KEY_LISTENING, snapshot.ports.len() as f64),
            Sample::new(KEY_PUBLIC, snapshot.public_ports().count() as f64),
        ];
        Ok(Snapshot::new(snapshot)
            .with_samples(samples)
            .with_events(events))
    }
}

async fn collect_firewall(transport: &dyn Transport) -> Option<Firewall> {
    if transport.sudo_mode() == SudoMode::None {
        return None;
    }
    let output = transport.exec_root(FIREWALL_SCRIPT).await.ok()?;
    firewall::parse(&output.stdout)
}

fn events_between(previous: &PortsSnapshot, current: &PortsSnapshot) -> Vec<Event> {
    let mut events = Vec::new();
    for port in &current.ports {
        let existed = previous.ports.iter().any(|p| p.same_socket(port));
        if !existed {
            let message = format!(
                "новый порт {}:{} ({})",
                port.address,
                port.port,
                port.process_label()
            );
            events.push(Event::new(ID, Severity::Info, message));
        }
    }
    for port in current.public_ports() {
        let exposed = port.firewall_allowed.is_none() || port.firewall_allowed == Some(true);
        let before = previous.ports.iter().find(|p| p.same_socket(port));
        let was_reachable = before.and_then(|p| p.reachable);
        if port.reachable == Some(true) && was_reachable == Some(false) && exposed {
            events.push(Event::new(
                ID,
                Severity::Info,
                format!("порт {} снова доступен снаружи", port.port),
            ));
        }
        if port.reachable == Some(false) && was_reachable == Some(true) {
            events.push(Event::new(
                ID,
                Severity::Warning,
                format!("порт {} стал недоступен снаружи", port.port),
            ));
        }
    }
    events
}
