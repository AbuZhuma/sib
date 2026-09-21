use std::sync::Arc;
use std::time::{Duration, Instant};

use asiba_core::{AppState, ServerState};
use asiba_incidents::{SystemAudit, system_audit};
use egui::{Context, Id};

const CACHE_KEY: &str = "system-audit-cache";
const MAX_AGE: Duration = Duration::from_secs(1);

#[derive(Clone)]
struct Cached {
    at: Instant,
    audit: Arc<SystemAudit>,
}

pub fn cached_audit(ctx: &Context, server: &ServerState, state: &AppState) -> Arc<SystemAudit> {
    let id = Id::new((CACHE_KEY, server.spec.id.as_str()));
    let cached: Option<Cached> = ctx.data(|d| d.get_temp(id));
    if let Some(cached) = cached.filter(|c| c.at.elapsed() < MAX_AGE) {
        return cached.audit;
    }
    let audit = Arc::new(system_audit(server, state));
    ctx.data_mut(|d| {
        d.insert_temp(
            id,
            Cached {
                at: Instant::now(),
                audit: Arc::clone(&audit),
            },
        )
    });
    audit
}
