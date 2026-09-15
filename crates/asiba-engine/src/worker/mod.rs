mod collect;
mod docs;
mod ping;
mod status;

use std::sync::{Arc, Mutex};
use std::time::Duration;

use asiba_core::{
    ConnectionStatus, Credentials, Intervals, ModuleId, ModuleRegistry, ModuleSettings,
    SETTING_ENABLED, SETTING_INTERVAL, Schedule, ServerSpec, SharedState, Transport,
    TransportError,
};
use asiba_storage::StorageWriter;
use asiba_transport::{HostKeyPolicy, connect};
use chrono::Utc;
use tokio::sync::{broadcast, watch};
use tokio::task::JoinSet;
use tokio::time::sleep;

use crate::backoff::Backoff;
use crate::engine::RepaintNotifier;
use crate::test_connection::detect_all;

pub use docs::DocWriter;
pub use ping::SERIES_KEY as PING_SERIES_KEY;

const REDETECT_INTERVAL: Duration = Duration::from_secs(600);
pub const BACKFILL_QUEUE: usize = 16;

pub type TransportSlot = Arc<Mutex<Option<Arc<dyn Transport>>>>;

pub struct WorkerContext {
    pub spec: ServerSpec,
    pub intervals: Intervals,
    pub credentials: Credentials,
    pub policy: HostKeyPolicy,
    pub registry: ModuleRegistry,
    pub state: SharedState,
    pub notify: RepaintNotifier,
    pub storage: Option<StorageWriter>,
    pub transport: TransportSlot,
    pub backfill: broadcast::Sender<ModuleId>,
    pub docs: docs::DocWriter,
}

fn collect_interval(
    schedule: Schedule,
    intervals: &Intervals,
    settings: &ModuleSettings,
) -> Option<Duration> {
    let base = schedule.interval_with(intervals)?;
    let custom = settings
        .get(SETTING_INTERVAL)
        .and_then(|v| v.trim().parse::<u64>().ok())
        .filter(|secs| *secs > 0)
        .map(Duration::from_secs);
    Some(custom.unwrap_or(base))
}

fn share_transport(ctx: &WorkerContext, transport: Option<Arc<dyn Transport>>) {
    if let Ok(mut slot) = ctx.transport.lock() {
        *slot = transport;
    }
}

pub async fn run(ctx: WorkerContext) {
    let ctx = Arc::new(ctx);
    let mut probes = JoinSet::new();
    probes.spawn(ping::run(Arc::clone(&ctx)));
    let mut backoff = Backoff::default();
    loop {
        status::set(&ctx, ConnectionStatus::Connecting);
        let outcome = connect(&ctx.spec, &ctx.credentials, ctx.policy.clone()).await;
        let reason = match outcome {
            Ok(transport) => {
                backoff.reset();
                share_transport(&ctx, Some(Arc::clone(&transport)));
                status::set(&ctx, ConnectionStatus::Online { since: Utc::now() });
                let reason = serve(&ctx, transport).await;
                share_transport(&ctx, None);
                reason
            }
            Err(TransportError::UnknownHostKey { fingerprint }) => {
                status::set(
                    &ctx,
                    ConnectionStatus::UntrustedHostKey {
                        fingerprint,
                        changed: false,
                    },
                );
                return;
            }
            Err(TransportError::HostKeyChanged { fingerprint }) => {
                status::set(
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
        status::go_offline(&ctx, reason, delay);
        sleep(delay).await;
    }
}

async fn serve(ctx: &Arc<WorkerContext>, transport: Arc<dyn Transport>) -> String {
    loop {
        let (lost_sender, mut lost) = watch::channel(None::<String>);
        let mut tasks = spawn_collectors(ctx, &transport, lost_sender).await;
        tokio::select! {
            _ = sleep(REDETECT_INTERVAL) => {
                tasks.abort_all();
            }
            _ = lost.changed() => {
                tasks.abort_all();
                let reason = lost.borrow().clone().unwrap_or_default();
                return reason;
            }
        }
    }
}

async fn spawn_collectors(
    ctx: &Arc<WorkerContext>,
    transport: &Arc<dyn Transport>,
    lost: watch::Sender<Option<String>>,
) -> JoinSet<()> {
    let detections = detect_all(transport.as_ref(), &ctx.registry).await;
    status::set_detections(ctx, &detections);
    let mut tasks = JoinSet::new();
    for detection in detections
        .into_iter()
        .filter(|d| d.availability.is_usable())
    {
        let Some(module) = ctx.registry.get(detection.id) else {
            continue;
        };
        let settings = ctx.spec.module_settings(detection.id.0);
        if settings
            .get(SETTING_ENABLED)
            .is_some_and(|v| v.trim() == "false")
        {
            continue;
        }
        let Some(interval) = collect_interval(module.schedule(), &ctx.intervals, &settings) else {
            continue;
        };
        let loop_ctx = collect::LoopContext {
            worker: Arc::clone(ctx),
            transport: Arc::clone(transport),
            module: Arc::clone(module),
            interval,
            lost: lost.clone(),
        };
        tasks.spawn(collect::run(loop_ctx));
    }
    tasks
}
