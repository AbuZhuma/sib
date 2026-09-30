use sib_core::Severity;
use sib_modules::anomalies::{self, AnomaliesSnapshot, Rates};

use crate::section::{DocContext, Section, SectionId};
use crate::write::{NONE, blank, bullet, field, heading, list, table};

pub struct AnomaliesSection;

fn snapshot<'a>(ctx: &'a DocContext<'_>) -> Option<&'a AnomaliesSnapshot> {
    ctx.server.data::<AnomaliesSnapshot>(anomalies::ID)
}

fn severity_name(severity: Severity) -> &'static str {
    match severity {
        Severity::Info => "info",
        Severity::Warning => "warning",
        Severity::Critical => "critical",
    }
}

fn headline(snapshot: &AnomaliesSnapshot, english: bool) -> &'static str {
    match (snapshot.is_under_attack(), snapshot.has_signs(), english) {
        (true, _, false) => "ATTACK",
        (false, true, false) => "suspicious",
        (false, false, false) => "no attack signs",
        (true, _, true) => "UNDER ATTACK",
        (false, true, true) => "suspicious",
        (false, false, true) => "calm",
    }
}

fn rate(value: Option<f64>) -> String {
    value
        .map(|v| format!("{v:.0}"))
        .unwrap_or_else(|| NONE.to_owned())
}

type RateField = fn(&Rates) -> f64;

const RATE_ROWS: [(&str, &str, RateField); 5] = [
    ("New connections/s", "new_connections_per_s", |r| {
        r.new_connections
    }),
    ("Failed connections/s", "failed_connections_per_s", |r| {
        r.failed_connections
    }),
    ("SYN cookies/s", "syncookies_per_s", |r| r.syncookies),
    ("Accept drops/s", "listen_drops_per_s", |r| r.listen_drops),
    ("UDP/s", "udp_per_s", |r| r.udp_in),
];

fn counters(snapshot: &AnomaliesSnapshot) -> Vec<(&'static str, &'static str, String)> {
    let rates = snapshot.rates;
    let mut rows = connection_rows(snapshot);
    rows.push((
        "Packets/s in / out",
        "packets_per_s_in_out",
        format!(
            "{} / {}",
            rate(rates.map(|r| r.pps_in)),
            rate(rates.map(|r| r.pps_out))
        ),
    ));
    for (label, key, field) in RATE_ROWS {
        rows.push((label, key, rate(rates.as_ref().map(field))));
    }
    rows.push((
        "Share of top 10 addresses",
        "top10_share_pct",
        format!("{:.0}%", snapshot.top_share_pct()),
    ));
    if let Some((count, max)) = snapshot.conntrack {
        rows.push(("conntrack", "conntrack", format!("{count} / {max}")));
    }
    rows
}

fn connection_rows(snapshot: &AnomaliesSnapshot) -> Vec<(&'static str, &'static str, String)> {
    vec![
        (
            "Half-open (SYN-RECV)",
            "syn_recv",
            snapshot.syn_recv().to_string(),
        ),
        (
            "Established",
            "established",
            snapshot.established().to_string(),
        ),
        (
            "Unique addresses",
            "distinct_peers",
            snapshot.distinct_peers.to_string(),
        ),
    ]
}

fn sign_rows(snapshot: &AnomaliesSnapshot) -> Vec<Vec<String>> {
    snapshot
        .signs
        .iter()
        .map(|s| {
            vec![
                severity_name(s.severity).to_owned(),
                s.kind.label().to_owned(),
                s.detail.clone(),
                s.peers.join(" "),
            ]
        })
        .collect()
}

fn peer_rows(snapshot: &AnomaliesSnapshot, ctx: &DocContext<'_>) -> Vec<Vec<String>> {
    let suspicious = snapshot.suspicious_peers();
    snapshot
        .top_peers
        .iter()
        .map(|p| {
            let flag = if suspicious.contains(&p.ip.as_str()) {
                "suspicious"
            } else {
                ""
            };
            vec![
                p.ip.clone(),
                ctx.state.country_of(&p.ip).unwrap_or(NONE).to_owned(),
                p.connections.to_string(),
                p.syn_recv.to_string(),
                flag.to_owned(),
            ]
        })
        .collect()
}

impl Section for AnomaliesSection {
    fn id(&self) -> SectionId {
        SectionId::Anomalies
    }

    fn is_available(&self, ctx: &DocContext<'_>) -> bool {
        snapshot(ctx).is_some()
    }

    fn human(&self, out: &mut String, ctx: &DocContext<'_>) {
        let Some(snapshot) = snapshot(ctx) else {
            return;
        };
        heading(out, "Anomalies and DDoS");
        bullet(out, "State", headline(snapshot, false));
        for (label, _, value) in counters(snapshot) {
            bullet(out, label, value);
        }
        blank(out);
        if !snapshot.signs.is_empty() {
            table(
                out,
                &["Level", "Sign", "Details", "IP"],
                &sign_rows(snapshot),
            );
        }
        table(
            out,
            &["IP", "Country", "Connections", "SYN-RECV", "Note"],
            &peer_rows(snapshot, ctx),
        );
    }

    fn llm(&self, out: &mut String, ctx: &DocContext<'_>) {
        let Some(snapshot) = snapshot(ctx) else {
            return;
        };
        heading(out, "Network anomalies / DDoS");
        field(out, "status", headline(snapshot, true));
        for (_, key, value) in counters(snapshot) {
            field(out, key, value);
        }
        if !snapshot.signs.is_empty() {
            field(out, "attack_signs", "severity | sign | detail | peers");
            list(out, "", &sign_rows(snapshot));
        }
        field(
            out,
            "top_peers",
            "ip | country | connections | syn_recv | flag",
        );
        list(out, "", &peer_rows(snapshot, ctx));
        blank(out);
    }
}
