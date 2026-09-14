use std::sync::{Arc, Mutex};
use std::time::Duration;

use asiba_core::{AuthMethod, Credentials, JumpHost, ServerSpec, TransportError};
use russh::client::{self, Handle};

use super::auth;
use super::handler::{ClientHandler, HostKeyVerdict};
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
    let Some(jump) = &spec.jump else {
        let session = connect_direct(&spec.host, spec.port, policy).await?;
        return Ok((
            finish_auth(session, &spec.user, &spec.auth, credentials).await?,
            None,
        ));
    };
    let jump_session = connect_jump(jump, credentials).await?;
    let session = connect_through(&jump_session, spec, policy).await?;
    Ok((
        finish_auth(session, &spec.user, &spec.auth, credentials).await?,
        Some(jump_session),
    ))
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
    let session = connect_direct(&jump.host, jump.port, HostKeyPolicy::KnownHostsOnly).await?;
    finish_auth(session, &jump.user, &AuthMethod::Auto, credentials).await
}

async fn connect_through(
    jump: &Session,
    spec: &ServerSpec,
    policy: HostKeyPolicy,
) -> Result<Session, TransportError> {
    let channel = jump
        .channel_open_direct_tcpip(spec.host.clone(), u32::from(spec.port), "127.0.0.1", 0)
        .await
        .map_err(|e| TransportError::Connect(format!("через jump host: {e}")))?;
    let handler = ClientHandler::new(&spec.host, spec.port, policy);
    let verdict = handler.verdict_slot();
    let attempt = client::connect_stream(config(), channel.into_stream(), handler);
    let result = tokio::time::timeout(CONNECT_TIMEOUT, attempt)
        .await
        .map_err(|_| TransportError::Timeout)?;
    result.map_err(|error| map_connect_error(error, &verdict))
}

async fn finish_auth(
    mut session: Session,
    user: &str,
    method: &AuthMethod,
    credentials: &Credentials,
) -> Result<Session, TransportError> {
    auth::authenticate(&mut session, user, method, credentials).await?;
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
