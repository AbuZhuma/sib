use super::model::{Ban, Jail};

const JAIL_MARKER: &str = "@@ ";

pub fn jails(raw: &str) -> Vec<Jail> {
    let mut jails = Vec::new();
    for block in raw.split(JAIL_MARKER).skip(1) {
        let Some((name, body)) = block.split_once('\n') else {
            continue;
        };
        jails.push(Jail {
            name: name.trim().to_owned(),
            currently_failed: status_number(body, "Currently failed:"),
            total_banned: status_number(body, "Total banned:"),
            banned: status_value(body, "Banned IP list:")
                .split_whitespace()
                .map(str::to_owned)
                .collect(),
        });
    }
    jails
}

fn status_value<'a>(body: &'a str, label: &str) -> &'a str {
    body.lines()
        .find_map(|l| l.split_once(label))
        .map(|(_, v)| v.trim())
        .unwrap_or_default()
}

fn status_number(body: &str, label: &str) -> u64 {
    status_value(body, label).parse().unwrap_or(0)
}

pub fn jail_bans(jails: &[Jail]) -> Vec<Ban> {
    jails
        .iter()
        .flat_map(|jail| {
            jail.banned.iter().map(|ip| Ban {
                ip: ip.clone(),
                source: format!("fail2ban/{}", jail.name),
                expires: None,
            })
        })
        .collect()
}

pub fn nft_bans(raw: &str) -> Vec<Ban> {
    let Some(start) = raw.find("elements = {") else {
        return Vec::new();
    };
    let body = &raw[start + "elements = {".len()..];
    let body = body.split('}').next().unwrap_or_default();
    body.split(',')
        .map(str::trim)
        .filter(|e| !e.is_empty())
        .filter_map(|element| {
            let mut tokens = element.split_whitespace();
            let ip = tokens.next()?;
            let expires = element
                .split_once("expires ")
                .map(|(_, e)| e.split_whitespace().next().unwrap_or_default().to_owned());
            Some(Ban {
                ip: ip.to_owned(),
                source: "nftables".to_owned(),
                expires,
            })
        })
        .collect()
}

pub fn iptables_bans(raw: &str) -> Vec<Ban> {
    raw.lines()
        .filter(|l| l.starts_with("-A SIB") && l.contains("-j DROP"))
        .filter_map(|l| l.split_once("-s ").map(|(_, rest)| rest))
        .filter_map(|rest| rest.split_whitespace().next())
        .map(|ip| Ban {
            ip: ip
                .trim_end_matches("/32")
                .trim_end_matches("/128")
                .to_owned(),
            source: "iptables".to_owned(),
            expires: None,
        })
        .collect()
}

pub fn ufw_bans(raw: &str) -> Vec<Ban> {
    raw.lines()
        .filter(|l| l.contains("DENY"))
        .filter_map(|l| l.split_whitespace().last())
        .filter(|ip| ip.chars().next().is_some_and(|c| c.is_ascii_hexdigit()))
        .map(|ip| Ban {
            ip: ip.to_owned(),
            source: "ufw".to_owned(),
            expires: None,
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn iptables_chain_lines_yield_bans() {
        let bans = iptables_bans("-N SIB\n-A SIB -s 10.0.0.5/32 -j DROP\n");
        assert_eq!(bans[0].ip, "10.0.0.5");
    }
}
