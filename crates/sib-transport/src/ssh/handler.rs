use std::sync::{Arc, Mutex};

use russh::client;
use russh::keys::known_hosts::learn_known_hosts;
use russh::keys::{HashAlg, PublicKey, PublicKeyOrCertificate, check_known_hosts};

use crate::connect::HostKeyPolicy;

#[derive(Debug, Clone)]
pub enum HostKeyVerdict {
    Unknown(String),
    Changed(String),
}

pub struct ClientHandler {
    host: String,
    port: u16,
    policy: HostKeyPolicy,
    verdict: Arc<Mutex<Option<HostKeyVerdict>>>,
}

impl ClientHandler {
    pub fn new(host: &str, port: u16, policy: HostKeyPolicy) -> Self {
        Self {
            host: host.to_owned(),
            port,
            policy,
            verdict: Arc::new(Mutex::new(None)),
        }
    }

    pub fn verdict_slot(&self) -> Arc<Mutex<Option<HostKeyVerdict>>> {
        Arc::clone(&self.verdict)
    }

    fn record(&self, verdict: HostKeyVerdict) {
        if let Ok(mut slot) = self.verdict.lock() {
            *slot = Some(verdict);
        }
    }

    fn accept_unknown(&self, key: &PublicKey, fingerprint: String) -> bool {
        let trusted =
            matches!(&self.policy, HostKeyPolicy::TrustFingerprint(f) if *f == fingerprint);
        if !trusted {
            self.record(HostKeyVerdict::Unknown(fingerprint));
            return false;
        }
        if let Err(error) = learn_known_hosts(&self.host, self.port, key) {
            tracing::warn!(host = %self.host, %error, "could not write the key to known_hosts");
        }
        true
    }
}

fn to_public_key(server_key: &PublicKeyOrCertificate) -> PublicKey {
    match server_key {
        PublicKeyOrCertificate::PublicKey { key, .. } => key.clone(),
        PublicKeyOrCertificate::Certificate(cert) => PublicKey::new(cert.public_key().clone(), ""),
    }
}

impl client::Handler for ClientHandler {
    type Error = russh::Error;

    async fn check_server_key(
        &mut self,
        server_key: &PublicKeyOrCertificate,
    ) -> Result<bool, Self::Error> {
        let key = to_public_key(server_key);
        let fingerprint = key.fingerprint(HashAlg::Sha256).to_string();
        match check_known_hosts(&self.host, self.port, &key) {
            Ok(true) => Ok(true),
            Ok(false) => Ok(self.accept_unknown(&key, fingerprint)),
            Err(russh::keys::Error::KeyChanged { .. }) => {
                self.record(HostKeyVerdict::Changed(fingerprint));
                Ok(false)
            }
            Err(error) => Err(error.into()),
        }
    }
}
