use std::sync::Arc;

use asiba_core::{Credentials, ServerSpec, Transport, TransportError};

use crate::local::LocalTransport;
use crate::ssh::SshTransport;

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum HostKeyPolicy {
    #[default]
    KnownHostsOnly,
    TrustFingerprint(String),
}

pub async fn connect(
    spec: &ServerSpec,
    credentials: &Credentials,
    policy: HostKeyPolicy,
) -> Result<Arc<dyn Transport>, TransportError> {
    if spec.is_local() {
        let local = LocalTransport::new(spec.sudo, credentials.sudo_password.clone());
        return Ok(Arc::new(local));
    }
    let ssh = SshTransport::connect(spec, credentials, policy).await?;
    Ok(Arc::new(ssh))
}
