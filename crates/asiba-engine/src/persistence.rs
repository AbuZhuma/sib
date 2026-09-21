use std::sync::Arc;

use asiba_config::{ConfigError, SecretStore, ServerStore};
use asiba_core::{Credentials, ServerId, ServerSpec};

#[derive(Clone)]
pub struct Persistence {
    servers: ServerStore,
    secrets: Arc<dyn SecretStore>,
}

impl Persistence {
    pub fn new(servers: ServerStore, secrets: Arc<dyn SecretStore>) -> Self {
        Self { servers, secrets }
    }

    pub fn load_all(&self) -> Result<Vec<(ServerSpec, Credentials)>, ConfigError> {
        let specs = self.servers.load_all()?;
        Ok(specs
            .into_iter()
            .map(|spec| self.with_credentials(spec))
            .collect())
    }

    fn with_credentials(&self, spec: ServerSpec) -> (ServerSpec, Credentials) {
        let credentials = match self.secrets.load_credentials(&spec.id) {
            Ok(credentials) => credentials,
            Err(error) => {
                tracing::warn!(server = %spec.id, %error, "секреты недоступны, вход без них");
                Credentials::default()
            }
        };
        (spec, credentials)
    }

    pub fn save(&self, spec: &ServerSpec, credentials: &Credentials) -> Result<(), ConfigError> {
        self.servers.save(spec)?;
        self.secrets.save_credentials(&spec.id, credentials)
    }

    pub fn doc_path(&self, id: &ServerId) -> std::path::PathBuf {
        self.servers.doc_path(id)
    }

    pub fn llm_doc_path(&self, id: &ServerId) -> std::path::PathBuf {
        self.servers.llm_doc_path(id)
    }

    pub fn rename(&self, previous: &ServerId, next: &ServerId) -> Result<(), ConfigError> {
        let credentials = self.secrets.load_credentials(previous)?;
        self.secrets.save_credentials(next, &credentials)?;
        self.servers.rename(previous, next)?;
        self.secrets.delete_all(previous)
    }

    pub fn remove(&self, id: &ServerId) -> Result<(), ConfigError> {
        self.servers.delete(id)?;
        self.secrets.delete_all(id)
    }
}
