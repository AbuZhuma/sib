use std::time::Duration;

use asiba_core::{ConnectionStatus, ModuleState};
use chrono::Utc;

use super::WorkerContext;
use crate::command::ModuleDetection;

pub fn set(ctx: &WorkerContext, status: ConnectionStatus) {
    if let Ok(mut state) = ctx.state.write()
        && let Some(server) = state.servers.get_mut(&ctx.spec.id)
    {
        server.connection = status;
    }
    (ctx.notify)();
}

pub fn go_offline(ctx: &WorkerContext, reason: String, delay: Duration) {
    let retry_at = Utc::now() + chrono::Duration::from_std(delay).unwrap_or_default();
    tracing::warn!(server = %ctx.spec.id, %reason, "сервер недоступен");
    set(ctx, ConnectionStatus::Offline { reason, retry_at });
}

pub fn set_detections(ctx: &WorkerContext, detections: &[ModuleDetection]) {
    if let Ok(mut state) = ctx.state.write()
        && let Some(server) = state.servers.get_mut(&ctx.spec.id)
    {
        for detection in detections {
            let entry = server
                .modules
                .entry(detection.id)
                .or_insert_with(|| ModuleState::detected(detection.availability.clone()));
            entry.availability = detection.availability.clone();
        }
    }
    (ctx.notify)();
}
