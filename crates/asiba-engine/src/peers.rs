use std::collections::BTreeMap;
use std::path::PathBuf;
use std::time::Duration;

use asiba_core::SharedState;
use asiba_modules::anomalies::{self, AnomaliesSnapshot};
use asiba_modules::security::{self, SecuritySnapshot};
use serde::{Deserialize, Serialize};
use tokio::time::{MissedTickBehavior, interval};

use crate::engine::RepaintNotifier;

const LOOKUP_INTERVAL: Duration = Duration::from_secs(30);
const BATCH_URL: &str = "http://ip-api.com/batch";
const BATCH_LIMIT: usize = 100;
const PER_SERVER: usize = 30;
const UNKNOWN: &str = "?";

#[derive(Serialize)]
struct BatchQuery {
    query: String,
    fields: &'static str,
}

#[derive(Deserialize)]
struct BatchAnswer {
    status: String,
    query: String,
    #[serde(rename = "countryCode")]
    country_code: Option<String>,
}

pub struct PeerLookup {
    pub cache: Option<PathBuf>,
    pub state: SharedState,
    pub notify: RepaintNotifier,
}

pub fn spawn(context: PeerLookup) {
    tokio::spawn(run(context));
}

async fn run(context: PeerLookup) {
    let mut ticker = interval(LOOKUP_INTERVAL);
    ticker.set_missed_tick_behavior(MissedTickBehavior::Delay);
    seed_from_cache(&context);
    loop {
        ticker.tick().await;
        let pending = pending_ips(&context.state);
        if pending.is_empty() {
            continue;
        }
        let resolved = tokio::task::spawn_blocking(move || lookup(&pending)).await;
        let Ok(resolved) = resolved else {
            continue;
        };
        if resolved.is_empty() {
            continue;
        }
        if let Ok(mut state) = context.state.write() {
            state.ip_countries.extend(resolved.clone());
        }
        persist(context.cache.as_deref(), &resolved);
        (context.notify)();
    }
}

fn seed_from_cache(context: &PeerLookup) {
    let cached = load(context.cache.as_deref());
    if cached.is_empty() {
        return;
    }
    if let Ok(mut state) = context.state.write() {
        state.ip_countries.extend(cached);
    }
}

fn pending_ips(state: &SharedState) -> Vec<String> {
    let Ok(state) = state.read() else {
        return Vec::new();
    };
    let mut ips = Vec::new();
    for server in state.servers.values() {
        if let Some(snapshot) = server.data::<SecuritySnapshot>(security::ID) {
            ips.extend(
                snapshot
                    .attackers
                    .iter()
                    .take(PER_SERVER)
                    .map(|a| a.ip.clone()),
            );
        }
        if let Some(snapshot) = server.data::<AnomaliesSnapshot>(anomalies::ID) {
            ips.extend(snapshot.top_peers.iter().map(|p| p.ip.clone()));
        }
    }
    ips.sort_unstable();
    ips.dedup();
    ips.retain(|ip| !state.ip_countries.contains_key(ip) && is_public(ip));
    ips.truncate(BATCH_LIMIT);
    ips
}

fn is_public(ip: &str) -> bool {
    match ip.parse::<std::net::IpAddr>() {
        Ok(std::net::IpAddr::V4(v4)) => {
            !(v4.is_private() || v4.is_loopback() || v4.is_link_local() || v4.is_unspecified())
        }
        Ok(std::net::IpAddr::V6(v6)) => !(v6.is_loopback() || v6.is_unique_local()),
        Err(_) => false,
    }
}

fn lookup(ips: &[String]) -> BTreeMap<String, String> {
    let queries: Vec<BatchQuery> = ips
        .iter()
        .map(|ip| BatchQuery {
            query: ip.clone(),
            fields: "status,query,countryCode",
        })
        .collect();
    let response = ureq::post(BATCH_URL)
        .send_json(&queries)
        .and_then(|mut r| r.body_mut().read_json::<Vec<BatchAnswer>>());
    match response {
        Ok(answers) => answers
            .into_iter()
            .map(|a| {
                let country = if a.status == "success" {
                    a.country_code.unwrap_or_else(|| UNKNOWN.to_owned())
                } else {
                    UNKNOWN.to_owned()
                };
                (a.query, country)
            })
            .collect(),
        Err(error) => {
            tracing::warn!(%error, "страны адресов не определены");
            BTreeMap::new()
        }
    }
}

fn countries_path(cache: Option<&std::path::Path>) -> Option<PathBuf> {
    cache.map(|p| p.with_file_name("countries.json"))
}

fn load(cache: Option<&std::path::Path>) -> BTreeMap<String, String> {
    countries_path(cache)
        .and_then(|path| std::fs::read_to_string(path).ok())
        .and_then(|raw| serde_json::from_str(&raw).ok())
        .unwrap_or_default()
}

fn persist(cache: Option<&std::path::Path>, fresh: &BTreeMap<String, String>) {
    let Some(path) = countries_path(cache) else {
        return;
    };
    let mut all = load(cache);
    all.extend(fresh.clone());
    if let Some(parent) = path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    if let Ok(raw) = serde_json::to_string(&all) {
        let _ = std::fs::write(path, raw);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn private_ips_are_not_looked_up() {
        assert!(!is_public("10.0.0.1"));
        assert!(!is_public("127.0.0.1"));
        assert!(is_public("203.0.113.9"));
        assert!(!is_public("garbage"));
    }
}
