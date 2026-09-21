use std::sync::Arc;
use std::time::{Duration, Instant};

use asiba_core::{PingStatus, Point, ServerSpec};
use chrono::Utc;
use tokio::net::TcpStream;
use tokio::time::{sleep, timeout};

use super::WorkerContext;

const PING_INTERVAL: Duration = Duration::from_secs(5);
const PING_TIMEOUT: Duration = Duration::from_secs(3);
pub const SERIES_KEY: &str = "ping.rtt_ms";

pub async fn run(ctx: Arc<WorkerContext>) {
    let mut lost_in_row = 0;
    let (host, port) = probe_address(&ctx.spec);
    loop {
        let (host, port) = asiba_transport::effective_address(&host, port);
        let rtt_ms = measure(&host, port).await;
        lost_in_row = if rtt_ms.is_some() { 0 } else { lost_in_row + 1 };
        record(&ctx, rtt_ms, lost_in_row);
        sleep(PING_INTERVAL).await;
    }
}

fn probe_address(spec: &ServerSpec) -> (String, u16) {
    match &spec.jump {
        Some(jump) => (jump.host.clone(), jump.port),
        None => (spec.host.clone(), spec.port),
    }
}

async fn measure(host: &str, port: u16) -> Option<f64> {
    let started = Instant::now();
    let attempt = timeout(PING_TIMEOUT, TcpStream::connect((host, port))).await;
    match attempt {
        Ok(Ok(_stream)) => Some(started.elapsed().as_secs_f64() * 1000.0),
        _ => None,
    }
}

fn record(ctx: &WorkerContext, rtt_ms: Option<f64>, lost_in_row: u32) {
    let at = Utc::now();
    if let Ok(mut state) = ctx.state.write()
        && let Some(server) = state.servers.get_mut(&ctx.spec.id)
    {
        server.ping = Some(PingStatus {
            rtt_ms,
            at,
            lost_in_row,
            is_jump_host: ctx.spec.jump.is_some(),
        });
        if let Some(value) = rtt_ms {
            server
                .series
                .entry(SERIES_KEY.to_owned())
                .or_default()
                .push(Point { at, value });
        }
    }
    (ctx.notify)();
}
