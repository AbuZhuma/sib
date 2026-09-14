use std::path::{Path, PathBuf};
use std::sync::Arc;

use asiba_core::{AuthMethod, Credentials, TransportError};
use russh::client::{AuthResult, Handle};
use russh::keys::agent::AgentIdentity;
use russh::keys::agent::client::AgentClient;
use russh::keys::{HashAlg, PrivateKeyWithHashAlg, load_secret_key};

use super::handler::ClientHandler;

const DEFAULT_KEY_NAMES: [&str; 3] = ["id_ed25519", "id_ecdsa", "id_rsa"];

type Session = Handle<ClientHandler>;

pub async fn authenticate(
    session: &mut Session,
    user: &str,
    method: &AuthMethod,
    credentials: &Credentials,
) -> Result<(), TransportError> {
    match method {
        AuthMethod::KeyFile { path, .. } => {
            with_key_file(session, user, path, credentials.passphrase.as_deref()).await
        }
        AuthMethod::Password => with_password(session, user, credentials).await,
        AuthMethod::Auto => auto(session, user, credentials).await,
    }
}

async fn auto(
    session: &mut Session,
    user: &str,
    credentials: &Credentials,
) -> Result<(), TransportError> {
    let mut reasons = Vec::new();
    match with_agent(session, user).await {
        Ok(()) => return Ok(()),
        Err(error) => reasons.push(format!("agent: {error}")),
    }
    for path in default_key_paths() {
        match with_key_file(session, user, &path, credentials.passphrase.as_deref()).await {
            Ok(()) => return Ok(()),
            Err(error) => reasons.push(format!("{}: {error}", path.display())),
        }
    }
    if credentials.password.is_some() {
        match with_password(session, user, credentials).await {
            Ok(()) => return Ok(()),
            Err(error) => reasons.push(format!("password: {error}")),
        }
    }
    Err(TransportError::Auth(reasons.join("; ")))
}

async fn with_password(
    session: &mut Session,
    user: &str,
    credentials: &Credentials,
) -> Result<(), TransportError> {
    let password = credentials
        .password
        .as_deref()
        .ok_or_else(|| TransportError::Auth("пароль не задан".to_owned()))?;
    let result = session
        .authenticate_password(user, password)
        .await
        .map_err(|e| TransportError::Auth(e.to_string()))?;
    check_result(result, "пароль отклонён")
}

async fn with_key_file(
    session: &mut Session,
    user: &str,
    path: &Path,
    passphrase: Option<&str>,
) -> Result<(), TransportError> {
    let key = load_secret_key(path, passphrase).map_err(|e| TransportError::Auth(e.to_string()))?;
    let hash_alg = best_rsa_hash(session).await;
    let key = PrivateKeyWithHashAlg::new(Arc::new(key), hash_alg);
    let result = session
        .authenticate_publickey(user, key)
        .await
        .map_err(|e| TransportError::Auth(e.to_string()))?;
    check_result(result, "ключ отклонён")
}

async fn with_agent(session: &mut Session, user: &str) -> Result<(), TransportError> {
    let mut agent = AgentClient::connect_env()
        .await
        .map_err(|e| TransportError::Auth(e.to_string()))?;
    let identities = agent
        .request_identities()
        .await
        .map_err(|e| TransportError::Auth(e.to_string()))?;
    let hash_alg = best_rsa_hash(session).await;
    for identity in identities {
        let AgentIdentity::PublicKey { key, .. } = identity else {
            continue;
        };
        let attempt = session
            .authenticate_publickey_with(user, key, hash_alg, &mut agent)
            .await;
        if matches!(attempt, Ok(AuthResult::Success)) {
            return Ok(());
        }
    }
    Err(TransportError::Auth(
        "ни один ключ из agent не подошёл".to_owned(),
    ))
}

async fn best_rsa_hash(session: &mut Session) -> Option<HashAlg> {
    session
        .best_supported_rsa_hash()
        .await
        .ok()
        .flatten()
        .flatten()
}

fn default_key_paths() -> Vec<PathBuf> {
    let Some(home) = std::env::var_os("HOME") else {
        return Vec::new();
    };
    let ssh_dir = PathBuf::from(home).join(".ssh");
    DEFAULT_KEY_NAMES
        .iter()
        .map(|name| ssh_dir.join(name))
        .filter(|path| path.is_file())
        .collect()
}

fn check_result(result: AuthResult, rejected: &str) -> Result<(), TransportError> {
    if result.success() {
        return Ok(());
    }
    Err(TransportError::Auth(rejected.to_owned()))
}
