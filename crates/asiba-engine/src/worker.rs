use std::collections::BTreeMap;
use std::time::Duration;

use asiba_core::{
    ConnectionStatus, Credentials, ModuleError, ModuleId, ModuleRegistry, ModuleState, ServerSpec,
    SharedState, Transport, TransportError,
};
use asiba_transport::{HostKeyPolicy, connect};
use chrono::Utc;
use tokio::time::{sleep, sleep_until};

use crate::backoff::Backoff;
use crate::engine::RepaintNotifier;
use crate::scheduler::{Due, Scheduler};
use crate::test_connection::detect_all;

const FAILURES_BEFORE_REDETECT: u32 = 3;

pub struct WorkerContext {
    pub spec: ServerSpec,
    pub credentials: Credentials,
    pub policy: HostKeyPolicy,
    pub registry: ModuleRegistry,
    pub state: SharedState,
    pub notify: RepaintNotifier,
}

pub async fn run(ctx: WorkerContext) {
    let mut backoff = Backoff::default();
    loop {
        set_status(&ctx, ConnectionStatus::Connecting);
        let outcome = connect(&ctx.spec, &ctx.credentials, ctx.policy.clone()).await;
        let reason = match outcome {
            Ok(transport) => {
                backoff.reset();
                set_status(&ctx, ConnectionStatus::Online { since: Utc::now() });
                serve(&ctx, transport.as_ref()).await
            }
            Err(TransportError::UnknownHostKey { fingerprint }) => {
                set_status(
                    &ctx,
                    ConnectionStatus::UntrustedHostKey {
                        fingerprint,
                        changed: false,
                    },
                );
                return;
            }
            Err(TransportError::HostKeyChanged { fingerprint }) => {
                set_status(
                    &ctx,
                    ConnectionStatus::UntrustedHostKey {
                        fingerprint,
                        changed: true,
                    },
                );
                return;
            }
            Err(error) => error.to_string(),
        };
        let delay = backoff.next();
        go_offline(&ctx, reason, delay);
        sleep(delay).await;
    }
}

async fn serve(ctx: &WorkerContext, transport: &dyn Transport) -> String {
    let mut scheduler = detect_and_schedule(ctx, transport).await;
    let mut failures: BTreeMap<ModuleId, u32> = BTreeMap::new();
    loop {
        let (at, due) = scheduler.next();
        sleep_until(at).await;
        match due {
            Due::Redetect => {
                scheduler = detect_and_schedule(ctx, transport).await;
                failures.clear();
            }
            Due::Collect(id) => {
                if let Err(reason) = collect_one(ctx, transport, id, &mut failures).await {
                    return reason;
                }
                if failures
                    .values()
                    .any(|count| *count >= FAILURES_BEFORE_REDETECT)
                {
                    scheduler.request_redetect();
                }
                scheduler.mark_collected(id);
            }
        }
    }
}

async fn detect_and_schedule(ctx: &WorkerContext, transport: &dyn Transport) -> Scheduler {
    let detections = detect_all(transport, &ctx.registry).await;
    let scheduled = detections
        .iter()
        .filter(|d| d.availability.is_usable())
        .filter_map(|d| Some((d.id, ctx.registry.get(d.id)?.schedule().interval()?)));
    let scheduler = Scheduler::new(scheduled);
    if let Ok(mut state) = ctx.state.write()
        && let Some(server) = state.servers.get_mut(&ctx.spec.id)
    {
        server.modules = detections
            .into_iter()
            .map(|d| (d.id, ModuleState::detected(d.availability)))
            .collect();
    }
    (ctx.notify)();
    scheduler
}

async fn collect_one(
    ctx: &WorkerContext,
    transport: &dyn Transport,
    id: ModuleId,
    failures: &mut BTreeMap<ModuleId, u32>,
) -> Result<(), String> {
    let Some(module) = ctx.registry.get(id) else {
        return Ok(());
    };
    let result = module.collect(transport).await;
    if let Err(ModuleError::Transport(TransportError::Disconnected(reason))) = &result {
        return Err(reason.clone());
    }
    if let Ok(mut state) = ctx.state.write() {
        let Some(server) = state.servers.get_mut(&ctx.spec.id) else {
            return Ok(());
        };
        let Some(module_state) = server.modules.get_mut(&id) else {
            return Ok(());
        };
        match result {
            Ok(snapshot) => {
                failures.remove(&id);
                let events = snapshot.events.clone();
                module_state.record_snapshot(snapshot);
                let server_id = ctx.spec.id.clone();
                state.push_events(events.into_iter().map(|e| e.for_server(server_id.clone())));
            }
            Err(error) => {
                *failures.entry(id).or_default() += 1;
                module_state.record_error(error);
            }
        }
    }
    (ctx.notify)();
    Ok(())
}

fn set_status(ctx: &WorkerContext, status: ConnectionStatus) {
    if let Ok(mut state) = ctx.state.write()
        && let Some(server) = state.servers.get_mut(&ctx.spec.id)
    {
        server.connection = status;
    }
    (ctx.notify)();
}

fn go_offline(ctx: &WorkerContext, reason: String, delay: Duration) {
    let retry_at = Utc::now() + chrono::Duration::from_std(delay).unwrap_or_default();
    tracing::warn!(server = %ctx.spec.id, %reason, "сервер недоступен");
    set_status(ctx, ConnectionStatus::Offline { reason, retry_at });
}
