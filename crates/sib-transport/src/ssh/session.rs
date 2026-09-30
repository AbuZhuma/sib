use std::sync::{Arc, Mutex};
use std::time::Duration;

use russh::client::{self, Handle};
use sib_core::{AuthMethod, Credentials, JumpHost, ServerSpec, TransportError};

use super::auth::{self, AuthPlan};
use super::handler::{ClientHandler, HostKeyVerdict};
use super::ssh_config;
use crate::connect::HostKeyPolicy;

const CONNECT_TIMEOUT: Duration = Duration::from_secs(15);
const KEEPALIVE_INTERVAL: Duration = Duration::from_secs(15);
const KEEPALIVE_MAX: usize = 3;

type Session = Handle<ClientHandler>;
type VerdictSlot = Arc<Mutex<Option<HostKeyVerdict>>>;

pub async fn establish(
    spec: &ServerSpec,
    credentials: &Credentials,
    policy: HostKeyPolicy,
) -> Result<(Session, Option<Session>), TransportError> {
    let resolved = ssh_config::resolve(&spec.host);
    let host = resolved
        .host_name
        .clone()
        .unwrap_or_else(|| spec.host.clone());
    let plan = AuthPlan {
        user: &spec.user,
        method: &spec.auth,
        credentials,
        extra_keys: &resolved.identity_files,
    };
    let Some(jump) = &spec.jump else {
        let session = connect_direct(&host, spec.port, policy).await?;
        return Ok((finish_auth(session, &plan).await?, None));
    };
    let jump_session = connect_jump(jump, credentials).await?;
    let session = connect_through(&jump_session, &host, spec.port, policy).await?;
    Ok((finish_auth(session, &plan).await?, Some(jump_session)))
}

fn config() -> Arc<client::Config> {
    Arc::new(client::Config {
        keepalive_interval: Some(KEEPALIVE_INTERVAL),
        keepalive_max: KEEPALIVE_MAX,
        nodelay: true,
        ..Default::default()
    })
}

async fn connect_direct(
    host: &str,
    port: u16,
    policy: HostKeyPolicy,
) -> Result<Session, TransportError> {
    let handler = ClientHandler::new(host, port, policy);
    let verdict = handler.verdict_slot();
    let attempt = client::connect(config(), (host, port), handler);
    let result = tokio::time::timeout(CONNECT_TIMEOUT, attempt)
        .await
        .map_err(|_| TransportError::Timeout)?;
    result.map_err(|error| map_connect_error(error, &verdict))
}

async fn connect_jump(
    jump: &JumpHost,
    credentials: &Credentials,
) -> Result<Session, TransportError> {
    let resolved = ssh_config::resolve(&jump.host);
    let host = resolved
        .host_name
        .clone()
        .unwrap_or_else(|| jump.host.clone());
    let session = connect_direct(&host, jump.port, HostKeyPolicy::KnownHostsOnly).await?;
    let plan = AuthPlan {
        user: &jump.user,
        method: &AuthMethod::Auto,
        credentials,
        extra_keys: &resolved.identity_files,
    };
    finish_auth(session, &plan).await
}

async fn connect_through(
    jump: &Session,
    host: &str,
    port: u16,
    policy: HostKeyPolicy,
) -> Result<Session, TransportError> {
    let channel = jump
        .channel_open_direct_tcpip(host.to_owned(), u32::from(port), "127.0.0.1", 0)
        .await
        .map_err(|e| TransportError::Connect(format!("через jump host: {e}")))?;
    let handler = ClientHandler::new(host, port, policy);
    let verdict = handler.verdict_slot();
    let attempt = client::connect_stream(config(), channel.into_stream(), handler);
    let result = tokio::time::timeout(CONNECT_TIMEOUT, attempt)
        .await
        .map_err(|_| TransportError::Timeout)?;
    result.map_err(|error| map_connect_error(error, &verdict))
}

async fn finish_auth(mut session: Session, plan: &AuthPlan<'_>) -> Result<Session, TransportError> {
    auth::authenticate(&mut session, plan).await?;
    Ok(session)
}

fn map_connect_error(error: russh::Error, verdict: &VerdictSlot) -> TransportError {
    let recorded = verdict.lock().ok().and_then(|slot| slot.clone());
    match recorded {
        Some(HostKeyVerdict::Unknown(fingerprint)) => {
            TransportError::UnknownHostKey { fingerprint }
        }
        Some(HostKeyVerdict::Changed(fingerprint)) => {
            TransportError::HostKeyChanged { fingerprint }
        }
        None => TransportError::Connect(error.to_string()),
    }
}
