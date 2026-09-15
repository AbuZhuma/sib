use std::sync::Arc;
use std::time::Duration;

use asiba_core::{
    CollectContext, Module, ModuleError, ModuleId, Snapshot, Transport, TransportError,
};
use asiba_storage::StoredSample;
use tokio::sync::{broadcast, watch};
use tokio::time::{MissedTickBehavior, interval};

use super::WorkerContext;

const FAILURES_BEFORE_GIVING_UP: u32 = 3;

pub struct LoopContext {
    pub worker: Arc<WorkerContext>,
    pub transport: Arc<dyn Transport>,
    pub module: Arc<dyn Module>,
    pub interval: Duration,
    pub lost: watch::Sender<Option<String>>,
}

pub async fn run(ctx: LoopContext) {
    let mut ticker = interval(ctx.interval);
    ticker.set_missed_tick_behavior(MissedTickBehavior::Delay);
    let mut requests = ctx.worker.backfill.subscribe();
    let mut previous: Option<Snapshot> = None;
    let mut failures = 0;
    loop {
        let is_backfill = tokio::select! {
            _ = ticker.tick() => false,
            request = requests.recv() => {
                if !is_own_request(&ctx, request) {
                    continue;
                }
                true
            }
        };
        match fetch(&ctx, previous.clone(), is_backfill).await {
            Ok(snapshot) => {
                failures = 0;
                record_snapshot(&ctx, &snapshot);
                previous = Some(snapshot);
            }
            Err(ModuleError::Transport(TransportError::Disconnected(reason))) => {
                let _ = ctx.lost.send(Some(reason));
                return;
            }
            Err(error) if is_backfill => record_error(&ctx, &error),
            Err(error) => {
                failures += 1;
                record_error(&ctx, &error);
                if failures >= FAILURES_BEFORE_GIVING_UP {
                    tracing::warn!(server = %ctx.worker.spec.id, module = %ctx.module.id(), %error, "модуль отключён до повторного detect");
                    return;
                }
            }
        }
    }
}

async fn fetch(
    ctx: &LoopContext,
    previous: Option<Snapshot>,
    is_backfill: bool,
) -> Result<Snapshot, ModuleError> {
    let context = CollectContext {
        previous,
        host: ctx.worker.spec.host.clone(),
        settings: ctx.worker.spec.module_settings(ctx.module.id().0),
    };
    if is_backfill {
        return ctx.module.backfill(ctx.transport.as_ref(), &context).await;
    }
    ctx.module.collect(ctx.transport.as_ref(), &context).await
}

fn is_own_request(
    ctx: &LoopContext,
    request: Result<ModuleId, broadcast::error::RecvError>,
) -> bool {
    matches!(request, Ok(module) if module == ctx.module.id())
}

fn record_snapshot(ctx: &LoopContext, snapshot: &Snapshot) {
    let worker = &ctx.worker;
    if let Ok(mut state) = worker.state.write() {
        let Some(server) = state.servers.get_mut(&worker.spec.id) else {
            return;
        };
        server.push_samples(snapshot.taken_at, &snapshot.samples);
        if let Some(module_state) = server.modules.get_mut(&ctx.module.id()) {
            module_state.record_snapshot(snapshot.clone());
        }
        let server_id = worker.spec.id.clone();
        let events: Vec<_> = snapshot
            .events
            .iter()
            .cloned()
            .map(|e| e.for_server(server_id.clone()))
            .collect();
        server.push_recent_events(events.iter().cloned());
        state.push_events(events);
    }
    persist_samples(ctx, snapshot);
    worker.docs.maybe_write(worker);
    (worker.notify)();
}

fn persist_samples(ctx: &LoopContext, snapshot: &Snapshot) {
    let Some(storage) = &ctx.worker.storage else {
        return;
    };
    let server = ctx.worker.spec.id.to_string();
    for sample in &snapshot.samples {
        let stored = StoredSample {
            server: server.clone(),
            key: sample.key.clone(),
            at: snapshot.taken_at,
            value: sample.value,
        };
        if storage.write(stored).is_err() {
            return;
        }
    }
}

fn record_error(ctx: &LoopContext, error: &ModuleError) {
    let worker = &ctx.worker;
    if let Ok(mut state) = worker.state.write()
        && let Some(server) = state.servers.get_mut(&worker.spec.id)
        && let Some(module_state) = server.modules.get_mut(&ctx.module.id())
    {
        module_state.record_error(error);
    }
    (worker.notify)();
}
