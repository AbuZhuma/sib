use asiba_modules::ports::{self, ListeningPort, PortsSnapshot};

use crate::section::{DocContext, Section, SectionId};
use crate::write::{NONE, blank, field, heading, list, table};

pub struct PortsSection;

const MAX_PORTS: usize = 60;

fn snapshot<'a>(ctx: &'a DocContext<'_>) -> Option<&'a PortsSnapshot> {
    ctx.server.data::<PortsSnapshot>(ports::ID)
}

fn flag(value: Option<bool>, yes: &str, no: &str) -> String {
    match value {
        Some(true) => yes.to_owned(),
        Some(false) => no.to_owned(),
        None => NONE.to_owned(),
    }
}

fn row(port: &ListeningPort, english: bool) -> Vec<String> {
    let (yes, no) = if english {
        ("yes", "no")
    } else {
        ("да", "нет")
    };
    vec![
        port.port.to_string(),
        port.protocol.label().to_owned(),
        port.address.clone(),
        port.process_label(),
        port.connections.to_string(),
        flag(port.reachable, yes, no),
        flag(port.firewall_allowed, yes, no),
    ]
}

fn is_local_only(port: &ListeningPort) -> bool {
    port.address.starts_with("127.") || port.address.starts_with("[::1]")
}

fn rows(snapshot: &PortsSnapshot, english: bool) -> Vec<Vec<String>> {
    snapshot
        .ports
        .iter()
        .filter(|p| !english || !is_local_only(p))
        .take(MAX_PORTS)
        .map(|p| row(p, english))
        .collect()
}

fn firewall_label(snapshot: &PortsSnapshot) -> String {
    match &snapshot.firewall {
        Some(firewall) => {
            let state = if firewall.active {
                "active"
            } else {
                "inactive"
            };
            let allowed: Vec<String> = firewall
                .allowed
                .iter()
                .map(|(protocol, port)| format!("{port}/{}", protocol.label()))
                .collect();
            format!(
                "{:?} {state}, allowed: {}",
                firewall.backend,
                allowed.join(" ")
            )
        }
        None => "unknown".to_owned(),
    }
}

impl Section for PortsSection {
    fn id(&self) -> SectionId {
        SectionId::Ports
    }

    fn is_available(&self, ctx: &DocContext<'_>) -> bool {
        snapshot(ctx).is_some()
    }

    fn human(&self, out: &mut String, ctx: &DocContext<'_>) {
        let Some(snapshot) = snapshot(ctx) else {
            return;
        };
        heading(out, "Порты");
        out.push_str(&format!("Файрвол: {}.\n\n", firewall_label(snapshot)));
        table(
            out,
            &[
                "Порт",
                "Протокол",
                "Адрес",
                "Процесс",
                "Соединений",
                "Снаружи",
                "В файрволе",
            ],
            &rows(snapshot, false),
        );
    }

    fn llm(&self, out: &mut String, ctx: &DocContext<'_>) {
        let Some(snapshot) = snapshot(ctx) else {
            return;
        };
        heading(out, "Listening ports");
        field(out, "firewall", firewall_label(snapshot));
        field(
            out,
            "public_ports",
            snapshot.public_ports().count().to_string(),
        );
        field(
            out,
            "loopback_only_ports",
            snapshot
                .ports
                .iter()
                .filter(|p| is_local_only(p))
                .count()
                .to_string(),
        );
        field(
            out,
            "ports",
            "port | protocol | address | process | connections | reachable_from_outside | firewall_allowed",
        );
        list(out, "", &rows(snapshot, true));
        blank(out);
    }
}
