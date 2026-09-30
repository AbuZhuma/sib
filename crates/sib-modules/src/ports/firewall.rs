use super::model::{Firewall, FirewallBackend, Protocol};
use crate::common::sections::Sections;

const SERVICE_PORTS: [(&str, u16); 6] = [
    ("ssh", 22),
    ("http", 80),
    ("https", 443),
    ("dhcpv6-client", 546),
    ("smtp", 25),
    ("dns", 53),
];

pub fn parse(raw: &str) -> Option<Firewall> {
    let sections = Sections::parse(raw);
    if let Some(ufw) = sections.get("ufw").filter(|s| s.contains("Status: active")) {
        return Some(Firewall {
            backend: FirewallBackend::Ufw,
            active: true,
            allowed: ufw_rules(ufw),
        });
    }
    if let Some(fw) = sections
        .get("firewalld")
        .filter(|s| s.contains("ports:") || s.contains("services:"))
    {
        return Some(Firewall {
            backend: FirewallBackend::Firewalld,
            active: true,
            allowed: firewalld_rules(fw),
        });
    }
    if let Some(nft) = sections.get("nft").filter(|s| s.contains("hook input")) {
        return Some(Firewall {
            backend: FirewallBackend::Nftables,
            active: true,
            allowed: dport_rules(nft),
        });
    }
    if let Some(ipt) = sections
        .get("iptables")
        .filter(|s| s.lines().any(|l| l.starts_with("-A INPUT")))
    {
        return Some(Firewall {
            backend: FirewallBackend::Iptables,
            active: true,
            allowed: dport_rules(ipt),
        });
    }
    None
}

fn ufw_rules(raw: &str) -> Vec<(Protocol, u16)> {
    raw.lines()
        .filter(|line| line.contains("ALLOW"))
        .flat_map(|line| spec_ports(line.split_whitespace().next().unwrap_or_default()))
        .collect()
}

fn firewalld_rules(raw: &str) -> Vec<(Protocol, u16)> {
    let mut allowed = Vec::new();
    for line in raw.lines().map(str::trim) {
        if let Some(rest) = line.strip_prefix("ports:") {
            allowed.extend(rest.split_whitespace().flat_map(spec_ports));
        }
        if let Some(rest) = line.strip_prefix("services:") {
            for service in rest.split_whitespace() {
                if let Some((_, port)) = SERVICE_PORTS.iter().find(|(name, _)| *name == service) {
                    allowed.push((Protocol::Tcp, *port));
                }
            }
        }
    }
    allowed
}

fn dport_rules(raw: &str) -> Vec<(Protocol, u16)> {
    let mut allowed = Vec::new();
    for line in raw
        .lines()
        .filter(|l| l.contains("accept") || l.contains("ACCEPT"))
    {
        let protocol = if line.contains("udp") {
            Protocol::Udp
        } else {
            Protocol::Tcp
        };
        let mut tokens = line.split_whitespace().peekable();
        while let Some(token) = tokens.next() {
            if !token.starts_with("dport") && !token.starts_with("--dport") {
                continue;
            }
            let rest: Vec<&str> = tokens
                .clone()
                .take_while(|t| !t.contains("accept") && !t.starts_with('-'))
                .collect();
            for number in rest
                .join(" ")
                .split(|c: char| !c.is_ascii_digit())
                .filter(|s| !s.is_empty())
            {
                if let Ok(port) = number.parse::<u16>() {
                    allowed.push((protocol, port));
                }
            }
            break;
        }
    }
    allowed
}

fn spec_ports(spec: &str) -> Vec<(Protocol, u16)> {
    let (ports, protocol) = match spec.rsplit_once('/') {
        Some((ports, "udp")) => (ports, Protocol::Udp),
        Some((ports, _)) => (ports, Protocol::Tcp),
        None => (spec, Protocol::Tcp),
    };
    ports
        .split(',')
        .filter_map(|p| p.trim().parse().ok())
        .map(|p| (protocol, p))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ufw_active_rules_are_parsed() {
        let raw = "###ufw\nStatus: active\n\nTo   Action  From\n22/tcp  ALLOW  Anywhere\n80,443/tcp  ALLOW  Anywhere\n";
        let fw = parse(raw).expect("firewall");
        assert_eq!(fw.backend, FirewallBackend::Ufw);
        assert_eq!(
            fw.allowed,
            vec![
                (Protocol::Tcp, 22),
                (Protocol::Tcp, 80),
                (Protocol::Tcp, 443)
            ]
        );
    }

    #[test]
    fn nft_dport_sets_are_parsed() {
        let raw = "###nft\ntable inet filter {\n chain input {\n  type filter hook input priority 0; policy drop;\n  tcp dport { 22, 443 } accept\n  udp dport 53 accept\n }\n}\n";
        let fw = parse(raw).expect("firewall");
        assert_eq!(fw.backend, FirewallBackend::Nftables);
        assert_eq!(
            fw.allowed,
            vec![
                (Protocol::Tcp, 22),
                (Protocol::Tcp, 443),
                (Protocol::Udp, 53)
            ]
        );
    }

    #[test]
    fn iptables_dport_is_parsed() {
        let raw = "###iptables\n-P INPUT DROP\n-A INPUT -p tcp -m tcp --dport 22 -j ACCEPT\n";
        let fw = parse(raw).expect("firewall");
        assert_eq!(fw.allowed, vec![(Protocol::Tcp, 22)]);
    }

    #[test]
    fn inactive_ufw_and_empty_nft_is_none() {
        assert_eq!(parse("###ufw\nStatus: inactive\n###nft\n"), None);
    }
}
