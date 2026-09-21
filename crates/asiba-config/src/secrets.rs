use asiba_core::{Credentials, ServerId};
use keyring::Entry;

use crate::error::ConfigError;

const SERVICE: &str = "asiba";
pub const AI_KEY_ACCOUNT: &str = "ai/api_key";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SecretKind {
    Password,
    Passphrase,
    SudoPassword,
}

impl SecretKind {
    fn suffix(self) -> &'static str {
        match self {
            Self::Password => "password",
            Self::Passphrase => "passphrase",
            Self::SudoPassword => "sudo",
        }
    }

    const ALL: [Self; 3] = [Self::Password, Self::Passphrase, Self::SudoPassword];
}

pub trait SecretStore: Send + Sync {
    fn get_named(&self, account: &str) -> Result<Option<String>, ConfigError>;

    fn set_named(&self, account: &str, value: &str) -> Result<(), ConfigError>;

    fn delete_named(&self, account: &str) -> Result<(), ConfigError>;

    fn get(&self, id: &ServerId, kind: SecretKind) -> Result<Option<String>, ConfigError> {
        self.get_named(&account(id, kind))
    }

    fn set(&self, id: &ServerId, kind: SecretKind, value: &str) -> Result<(), ConfigError> {
        self.set_named(&account(id, kind), value)
    }

    fn delete(&self, id: &ServerId, kind: SecretKind) -> Result<(), ConfigError> {
        self.delete_named(&account(id, kind))
    }

    fn set_or_delete_named(&self, account: &str, value: &str) -> Result<(), ConfigError> {
        if value.trim().is_empty() {
            return self.delete_named(account);
        }
        self.set_named(account, value.trim())
    }

    fn load_credentials(&self, id: &ServerId) -> Result<Credentials, ConfigError> {
        Ok(Credentials {
            password: self.get(id, SecretKind::Password)?,
            passphrase: self.get(id, SecretKind::Passphrase)?,
            sudo_password: self.get(id, SecretKind::SudoPassword)?,
        })
    }

    fn save_credentials(&self, id: &ServerId, creds: &Credentials) -> Result<(), ConfigError> {
        self.set_or_delete(id, SecretKind::Password, creds.password.as_deref())?;
        self.set_or_delete(id, SecretKind::Passphrase, creds.passphrase.as_deref())?;
        self.set_or_delete(id, SecretKind::SudoPassword, creds.sudo_password.as_deref())
    }

    fn delete_all(&self, id: &ServerId) -> Result<(), ConfigError> {
        SecretKind::ALL
            .iter()
            .try_for_each(|kind| self.delete(id, *kind))
    }

    fn set_or_delete(
        &self,
        id: &ServerId,
        kind: SecretKind,
        value: Option<&str>,
    ) -> Result<(), ConfigError> {
        match value.filter(|v| !v.is_empty()) {
            Some(value) => self.set(id, kind, value),
            None => self.delete(id, kind),
        }
    }
}

fn account(id: &ServerId, kind: SecretKind) -> String {
    format!("{id}/{}", kind.suffix())
}

#[derive(Debug, Default, Clone)]
pub struct KeyringSecretStore;

impl KeyringSecretStore {
    fn entry(account: &str) -> Result<Entry, ConfigError> {
        Entry::new(SERVICE, account).map_err(|e| ConfigError::Secrets(e.to_string()))
    }
}

impl SecretStore for KeyringSecretStore {
    fn get_named(&self, account: &str) -> Result<Option<String>, ConfigError> {
        match Self::entry(account)?.get_password() {
            Ok(value) => Ok(Some(value)),
            Err(keyring::Error::NoEntry) => Ok(None),
            Err(error) => Err(ConfigError::Secrets(error.to_string())),
        }
    }

    fn set_named(&self, account: &str, value: &str) -> Result<(), ConfigError> {
        Self::entry(account)?
            .set_password(value)
            .map_err(|e| ConfigError::Secrets(e.to_string()))
    }

    fn delete_named(&self, account: &str) -> Result<(), ConfigError> {
        match Self::entry(account)?.delete_credential() {
            Ok(()) | Err(keyring::Error::NoEntry) => Ok(()),
            Err(error) => Err(ConfigError::Secrets(error.to_string())),
        }
    }
}
