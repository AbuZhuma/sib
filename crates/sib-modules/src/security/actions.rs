use sib_core::{ActionOutcome, ActionRequest, ActionSpec, Danger, ModuleError, Transport};

use super::ID;
use super::model::BanBackend;
use crate::common::root::{exec_as_root, require_success, validate_ip, validate_name};

pub const ACTION_BAN: &str = "ban";
pub const ACTION_UNBAN: &str = "unban";
pub const PERMANENT: &str = "";

pub const SPEC_BAN: ActionSpec = ActionSpec::new(ID, ACTION_BAN, "Ban the IP", Danger::Normal);
pub const SPEC_UNBAN: ActionSpec =
    ActionSpec::new(ID, ACTION_UNBAN, "Unban the IP", Danger::Normal);
pub const SPECS: [ActionSpec; 2] = [SPEC_BAN, SPEC_UNBAN];

const NFT_SETUP: &str = "nft list table inet sib >/dev/null 2>&1 || { nft add table inet sib && nft add set inet sib bans '{ type ipv4_addr; flags timeout; }' && nft add set inet sib bans6 '{ type ipv6_addr; flags timeout; }' && nft add chain inet sib input '{ type filter hook input priority -10; policy accept; }' && nft add rule inet sib input ip saddr @bans drop && nft add rule inet sib input ip6 saddr @bans6 drop; }";
const IPTABLES_SETUP: &str = "{ iptables -N SIB 2>/dev/null; iptables -C INPUT -j SIB 2>/dev/null || iptables -I INPUT -j SIB; }";
const IP6TABLES_SETUP: &str = "{ ip6tables -N SIB 2>/dev/null; ip6tables -C INPUT -j SIB 2>/dev/null || ip6tables -I INPUT -j SIB; }";
const OWN_ADDRESS_PROBE: &str = "printf '%s\\n' \"${SSH_CLIENT%% *}\" \"${SSH_CONNECTION%% *}\"";
const DETECT_BACKEND: &str = "for t in fail2ban-client nft iptables ufw; do command -v \"$t\" >/dev/null && echo \"$t\"; done; fail2ban-client status 2>/dev/null | sed -n 's/.*Jail list:[[:space:]]*//p'";

pub async fn perform(
    transport: &dyn Transport,
    request: &ActionRequest,
) -> Result<ActionOutcome, ModuleError> {
    let ip = validate_ip(&request.target)?;
    match request.kind.as_str() {
        ACTION_BAN => {
            ban(
                transport,
                &ip,
                request.argument.as_deref().unwrap_or(PERMANENT),
            )
            .await
        }
        ACTION_UNBAN => unban(transport, &ip, request.argument.as_deref()).await,
        other => Err(ModuleError::UnsupportedAction(other.to_owned())),
    }
}

struct Backend {
    kind: BanBackend,
    jail: Option<String>,
}

async fn detect(transport: &dyn Transport) -> Result<Backend, ModuleError> {
    let output = exec_as_root(transport, DETECT_BACKEND).await?;
    let mut tools = Vec::new();
    let mut jail = None;
    for line in output.stdout.lines().map(str::trim) {
        match line {
            "fail2ban-client" | "nft" | "iptables" | "ufw" => tools.push(line.to_owned()),
            jails if !jails.is_empty() => jail = preferred_jail(jails),
            _ => {}
        }
    }
    let has = |name: &str| tools.iter().any(|t| t == name);
    let kind = if has("fail2ban-client") && jail.is_some() {
        BanBackend::Fail2ban
    } else if has("nft") {
        BanBackend::Nftables
    } else if has("iptables") {
        BanBackend::Iptables
    } else if has("ufw") {
        BanBackend::Ufw
    } else {
        return Err(ModuleError::ActionFailed(
            "no fail2ban, nft, iptables or ufw".to_owned(),
        ));
    };
    Ok(Backend { kind, jail })
}

fn preferred_jail(list: &str) -> Option<String> {
    let jails: Vec<&str> = list
        .split(',')
        .map(str::trim)
        .filter(|j| !j.is_empty())
        .collect();
    jails
        .iter()
        .find(|j| j.contains("ssh"))
        .or(jails.first())
        .map(|j| (*j).to_owned())
}

async fn own_addresses(transport: &dyn Transport) -> Result<Vec<String>, ModuleError> {
    let output = transport.exec(OWN_ADDRESS_PROBE).await?;
    Ok(output
        .stdout
        .lines()
        .filter_map(|line| validate_ip(line).ok())
        .collect())
}

fn ensure_not_own(ip: &str, own: &[String]) -> Result<(), ModuleError> {
    if own.iter().any(|address| address == ip) {
        return Err(ModuleError::ActionFailed(format!(
            "{ip} is the address Sib is connected from, banning it would cut off access"
        )));
    }
    Ok(())
}

async fn ban(
    transport: &dyn Transport,
    ip: &str,
    duration: &str,
) -> Result<ActionOutcome, ModuleError> {
    ensure_not_own(ip, &own_addresses(transport).await?)?;
    let backend = detect(transport).await?;
    let duration = validate_duration(duration)?;
    let command = ban_command(&backend, ip, duration.as_deref())?;
    require_success(exec_as_root(transport, &command).await?)?;
    Ok(ActionOutcome::new(ban_message(
        &backend,
        ip,
        duration.as_deref(),
    )))
}

fn ban_command(backend: &Backend, ip: &str, duration: Option<&str>) -> Result<String, ModuleError> {
    Ok(match backend.kind {
        BanBackend::Fail2ban => {
            let jail = validate_name(backend.jail.as_deref().unwrap_or_default())?;
            format!("fail2ban-client set {jail} banip {ip}")
        }
        BanBackend::Nftables => {
            let timeout = duration
                .map(|d| format!(" timeout {d}"))
                .unwrap_or_default();
            format!(
                "{NFT_SETUP} && nft add element inet sib {} '{{ {ip}{timeout} }}'",
                nft_set(ip)
            )
        }
        BanBackend::Iptables => {
            let (setup, binary) = iptables_for(ip);
            format!("{setup} && {binary} -A SIB -s {ip} -j DROP")
        }
        BanBackend::Ufw => format!("ufw insert 1 deny from {ip}"),
    })
}

fn ban_message(backend: &Backend, ip: &str, duration: Option<&str>) -> String {
    let label = backend.kind.label();
    match duration {
        None => format!("{ip} banned through {label} forever"),
        Some(period) if backend.kind.supports_timeout() => {
            format!("{ip} banned through {label} for {period}")
        }
        Some(_) => format!("{ip} banned through {label} forever ({label} has no time limit)"),
    }
}

async fn unban(
    transport: &dyn Transport,
    ip: &str,
    source: Option<&str>,
) -> Result<ActionOutcome, ModuleError> {
    let backend = match source {
        Some(source) if source.starts_with("fail2ban/") => Backend {
            kind: BanBackend::Fail2ban,
            jail: Some(source.trim_start_matches("fail2ban/").to_owned()),
        },
        Some("nftables") => Backend {
            kind: BanBackend::Nftables,
            jail: None,
        },
        Some("iptables") => Backend {
            kind: BanBackend::Iptables,
            jail: None,
        },
        Some("ufw") => Backend {
            kind: BanBackend::Ufw,
            jail: None,
        },
        _ => detect(transport).await?,
    };
    let command = unban_command(&backend, ip)?;
    require_success(exec_as_root(transport, &command).await?)?;
    Ok(ActionOutcome::new(format!(
        "{ip} unbanned through {}",
        backend.kind.label()
    )))
}

fn unban_command(backend: &Backend, ip: &str) -> Result<String, ModuleError> {
    Ok(match backend.kind {
        BanBackend::Fail2ban => match &backend.jail {
            Some(jail) => format!("fail2ban-client set {} unbanip {ip}", validate_name(jail)?),
            None => format!("fail2ban-client unban {ip}"),
        },
        BanBackend::Nftables => {
            format!("nft delete element inet sib {} '{{ {ip} }}'", nft_set(ip))
        }
        BanBackend::Iptables => {
            let (_, binary) = iptables_for(ip);
            format!("{binary} -D SIB -s {ip} -j DROP")
        }
        BanBackend::Ufw => format!("ufw delete deny from {ip}"),
    })
}

fn nft_set(ip: &str) -> &'static str {
    if ip.contains(':') { "bans6" } else { "bans" }
}

fn iptables_for(ip: &str) -> (&'static str, &'static str) {
    if ip.contains(':') {
        (IP6TABLES_SETUP, "ip6tables")
    } else {
        (IPTABLES_SETUP, "iptables")
    }
}

fn validate_duration(value: &str) -> Result<Option<String>, ModuleError> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return Ok(None);
    }
    let digits = trimmed.trim_end_matches(['s', 'm', 'h', 'd']);
    let has_unit = digits.len() < trimmed.len();
    if digits.is_empty() || !has_unit || !digits.chars().all(|c| c.is_ascii_digit()) {
        return Err(ModuleError::ActionFailed(format!(
            "the period must look like 30m, 12h or 7d, got {trimmed}"
        )));
    }
    Ok(Some(trimmed.to_owned()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn preferred_jail_picks_ssh_jail() {
        assert_eq!(
            preferred_jail("nginx-http-auth, sshd").as_deref(),
            Some("sshd")
        );
        assert_eq!(preferred_jail("nginx").as_deref(), Some("nginx"));
        assert_eq!(preferred_jail(""), None);
    }

    #[test]
    fn validate_duration_accepts_units_and_rejects_garbage() {
        assert_eq!(
            validate_duration("24h").expect("ok").as_deref(),
            Some("24h")
        );
        assert_eq!(validate_duration("").expect("ok"), None);
        assert!(validate_duration("24").is_err());
        assert!(validate_duration("1h; reboot").is_err());
    }

    #[test]
    fn ban_command_uses_timeout_only_for_nftables() {
        let nft = Backend {
            kind: BanBackend::Nftables,
            jail: None,
        };
        let command = ban_command(&nft, "1.2.3.4", Some("12h")).expect("command");
        assert!(command.ends_with("nft add element inet sib bans '{ 1.2.3.4 timeout 12h }'"));
        let jail = Backend {
            kind: BanBackend::Fail2ban,
            jail: Some("sshd".to_owned()),
        };
        assert_eq!(
            ban_command(&jail, "1.2.3.4", Some("12h")).expect("command"),
            "fail2ban-client set sshd banip 1.2.3.4"
        );
    }

    #[test]
    fn ensure_not_own_rejects_session_address_and_allows_others() {
        let own = vec!["203.0.113.5".to_owned()];
        assert!(ensure_not_own("203.0.113.5", &own).is_err());
        assert!(ensure_not_own("198.51.100.9", &own).is_ok());
        assert!(ensure_not_own("198.51.100.9", &[]).is_ok());
    }

    #[test]
    fn nft_set_depends_on_ip_family() {
        assert_eq!(nft_set("1.2.3.4"), "bans");
        assert_eq!(nft_set("2001:db8::1"), "bans6");
    }
}
