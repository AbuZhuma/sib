use std::collections::BTreeMap;
use std::net::{IpAddr, ToSocketAddrs};
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

use chrono::Utc;
use serde::{Deserialize, Serialize};
use sib_core::{Location, LocationSource, ServerId, ServerSpec, SharedState};

use crate::address;
use crate::engine::RepaintNotifier;

const SELF_KEY: &str = "self";
const CACHE_TTL_SECS: i64 = 30 * 24 * 3600;
const LOOKUP_URL: &str = "http://ip-api.com/json/";
const LOOKUP_FIELDS: &str = "status,message,country,city,lat,lon,isp,org";
const LANGUAGE: &str = "ru";

static CACHE_LOCK: Mutex<()> = Mutex::new(());

#[derive(Debug, Clone, Serialize, Deserialize)]
struct CachedLocation {
    lat: f64,
    lon: f64,
    label: String,
    provider: String,
    at: i64,
}

impl CachedLocation {
    fn to_location(&self) -> Location {
        Location {
            lat: self.lat,
            lon: self.lon,
            label: self.label.clone(),
            provider: self.provider.clone(),
            source: LocationSource::Lookup,
        }
    }
}

#[derive(Debug, Deserialize)]
struct LookupResponse {
    status: String,
    message: Option<String>,
    country: Option<String>,
    city: Option<String>,
    lat: Option<f64>,
    lon: Option<f64>,
    isp: Option<String>,
    org: Option<String>,
}

pub struct GeoRequest {
    pub cache: Option<PathBuf>,
    pub enabled: Arc<AtomicBool>,
    pub state: SharedState,
    pub notify: RepaintNotifier,
}

impl GeoRequest {
    fn is_enabled(&self) -> bool {
        self.enabled.load(Ordering::Relaxed)
    }
}

pub fn resolve_server(request: GeoRequest, spec: ServerSpec) {
    tokio::task::spawn_blocking(move || {
        let location = match &spec.location {
            Some(manual) => Some(Location::manual(manual)),
            None if request.is_enabled() => {
                lookup_key(&spec).and_then(|key| locate(request.cache.as_deref(), &key))
            }
            None => None,
        };
        store_server(&request.state, &spec.id, location);
        (request.notify)();
    });
}

pub fn resolve_self(request: GeoRequest) {
    if !request.is_enabled() {
        return;
    }
    tokio::task::spawn_blocking(move || {
        let location = locate(request.cache.as_deref(), SELF_KEY);
        if let Ok(mut state) = request.state.write() {
            state.self_location = location;
        }
        (request.notify)();
    });
}

fn store_server(state: &SharedState, id: &ServerId, location: Option<Location>) {
    if let Ok(mut state) = state.write()
        && let Some(server) = state.servers.get_mut(id)
    {
        server.location = location;
    }
}

fn lookup_key(spec: &ServerSpec) -> Option<String> {
    let (host, port) = sib_transport::effective_address(&spec.host, spec.port);
    let ip = host
        .parse::<IpAddr>()
        .ok()
        .or_else(|| resolve_host(&host, port))?;
    if address::is_private(ip) {
        return Some(SELF_KEY.to_owned());
    }
    Some(ip.to_string())
}

fn resolve_host(host: &str, port: u16) -> Option<IpAddr> {
    (host, port)
        .to_socket_addrs()
        .ok()?
        .map(|addr| addr.ip())
        .find(|ip| ip.is_ipv4())
}

fn locate(cache_path: Option<&std::path::Path>, key: &str) -> Option<Location> {
    let mut cache = load_cache(cache_path);
    let now = Utc::now().timestamp();
    if let Some(entry) = cache.get(key).filter(|e| now - e.at < CACHE_TTL_SECS) {
        return Some(entry.to_location());
    }
    let entry = fetch(key)?;
    let location = entry.to_location();
    cache.insert(key.to_owned(), entry);
    save_cache(cache_path, &cache);
    Some(location)
}

fn fetch(key: &str) -> Option<CachedLocation> {
    let target = if key == SELF_KEY { "" } else { key };
    let url = format!("{LOOKUP_URL}{target}?fields={LOOKUP_FIELDS}&lang={LANGUAGE}");
    let response = ureq::get(&url)
        .call()
        .and_then(|mut r| r.body_mut().read_json::<LookupResponse>());
    let parsed = match response {
        Ok(parsed) => parsed,
        Err(error) => {
            tracing::warn!(%error, key, "geolocation is not available");
            return None;
        }
    };
    if parsed.status != "success" {
        tracing::warn!(key, message = ?parsed.message, "geolocation request rejected");
        return None;
    }
    let label = [parsed.city, parsed.country]
        .into_iter()
        .flatten()
        .filter(|s| !s.is_empty())
        .collect::<Vec<_>>()
        .join(", ");
    Some(CachedLocation {
        lat: parsed.lat?,
        lon: parsed.lon?,
        label,
        provider: parsed.org.or(parsed.isp).unwrap_or_default(),
        at: Utc::now().timestamp(),
    })
}

fn load_cache(path: Option<&std::path::Path>) -> BTreeMap<String, CachedLocation> {
    let Some(path) = path else {
        return BTreeMap::new();
    };
    let _guard = CACHE_LOCK.lock();
    std::fs::read_to_string(path)
        .ok()
        .and_then(|raw| serde_json::from_str(&raw).ok())
        .unwrap_or_default()
}

fn save_cache(path: Option<&std::path::Path>, cache: &BTreeMap<String, CachedLocation>) {
    let Some(path) = path else {
        return;
    };
    let _guard = CACHE_LOCK.lock();
    if let Some(parent) = path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    if let Ok(raw) = serde_json::to_string_pretty(cache) {
        let _ = std::fs::write(path, raw);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cache_roundtrips_through_json_file() {
        let dir = std::env::temp_dir().join(format!("sib-geo-{}", std::process::id()));
        let path = dir.join("geo.json");
        let mut cache = BTreeMap::new();
        cache.insert(
            "1.1.1.1".to_owned(),
            CachedLocation {
                lat: 1.0,
                lon: 2.0,
                label: "x".to_owned(),
                provider: "y".to_owned(),
                at: 5,
            },
        );
        save_cache(Some(&path), &cache);
        let loaded = load_cache(Some(&path));
        assert_eq!(loaded.get("1.1.1.1").map(|e| e.lat), Some(1.0));
        let _ = std::fs::remove_dir_all(dir);
    }
}
