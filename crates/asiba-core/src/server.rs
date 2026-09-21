use std::collections::BTreeMap;
use std::fmt;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};

const MAX_NAME_LEN: usize = 32;

#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(transparent)]
pub struct ServerId(String);

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum InvalidServerName {
    #[error("имя не может быть пустым")]
    Empty,
    #[error("имя длиннее {MAX_NAME_LEN} символов")]
    TooLong,
    #[error("допустимы только строчные латинские буквы, цифры и дефис")]
    BadChar,
    #[error("имя не может начинаться или заканчиваться дефисом")]
    EdgeHyphen,
}

impl ServerId {
    pub fn parse(raw: &str) -> Result<Self, InvalidServerName> {
        let name = raw.trim();
        if name.is_empty() {
            return Err(InvalidServerName::Empty);
        }
        if name.len() > MAX_NAME_LEN {
            return Err(InvalidServerName::TooLong);
        }
        let allowed = |c: char| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-';
        if !name.chars().all(allowed) {
            return Err(InvalidServerName::BadChar);
        }
        if name.starts_with('-') || name.ends_with('-') {
            return Err(InvalidServerName::EdgeHyphen);
        }
        Ok(Self(name.to_owned()))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for ServerId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ServerSpec {
    pub id: ServerId,
    pub host: String,
    pub port: u16,
    pub user: String,
    pub auth: AuthMethod,
    pub jump: Option<JumpHost>,
    pub sudo: SudoMode,
    pub description: ServerDescription,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub location: Option<ManualLocation>,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub modules: BTreeMap<String, ModuleSettings>,
}

pub type ModuleSettings = BTreeMap<String, String>;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ManualLocation {
    pub lat: f64,
    pub lon: f64,
    #[serde(default)]
    pub label: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LocationSource {
    Manual,
    Lookup,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Location {
    pub lat: f64,
    pub lon: f64,
    pub label: String,
    pub provider: String,
    pub source: LocationSource,
}

impl Location {
    pub fn manual(manual: &ManualLocation) -> Self {
        Self {
            lat: manual.lat,
            lon: manual.lon,
            label: manual.label.clone(),
            provider: String::new(),
            source: LocationSource::Manual,
        }
    }
}

impl ServerSpec {
    pub fn module_settings(&self, module: &str) -> ModuleSettings {
        self.modules.get(module).cloned().unwrap_or_default()
    }

    pub fn is_local(&self) -> bool {
        matches!(self.host.as_str(), "localhost" | "127.0.0.1" | "::1")
    }

    pub fn external_host(&self) -> String {
        if self.is_local() {
            return String::new();
        }
        self.host.clone()
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum AuthMethod {
    Auto,
    KeyFile { path: PathBuf, has_passphrase: bool },
    Password,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct JumpHost {
    pub host: String,
    pub port: u16,
    pub user: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SudoMode {
    #[default]
    None,
    Passwordless,
    WithPassword,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Environment {
    #[default]
    Production,
    Staging,
    Development,
    Other,
}

#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct ServerDescription {
    pub project: String,
    pub purpose: String,
    pub notes: String,
    pub environment: Environment,
    pub tags: Vec<String>,
    pub owner: String,
    pub links: Vec<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Credentials {
    pub password: Option<String>,
    pub passphrase: Option<String>,
    pub sudo_password: Option<String>,
}

impl Credentials {
    pub fn fill_missing_from(mut self, other: &Self) -> Self {
        self.password = self.password.or_else(|| other.password.clone());
        self.passphrase = self.passphrase.or_else(|| other.passphrase.clone());
        self.sudo_password = self.sudo_password.or_else(|| other.sudo_password.clone());
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_valid_name_returns_id() {
        assert_eq!(
            ServerId::parse(" neo-01 ").map(|id| id.0),
            Ok("neo-01".to_owned())
        );
    }

    #[test]
    fn parse_uppercase_rejects_bad_char() {
        assert_eq!(ServerId::parse("Neo"), Err(InvalidServerName::BadChar));
    }

    #[test]
    fn parse_leading_hyphen_rejects_edge() {
        assert_eq!(ServerId::parse("-neo"), Err(InvalidServerName::EdgeHyphen));
    }

    #[test]
    fn fill_missing_keeps_present_and_takes_absent() {
        let saved = Credentials {
            password: Some("old".into()),
            ..Default::default()
        };
        let fresh = Credentials {
            passphrase: Some("pp".into()),
            ..Default::default()
        };
        let merged = fresh.fill_missing_from(&saved);
        assert_eq!(merged.password.as_deref(), Some("old"));
        assert_eq!(merged.passphrase.as_deref(), Some("pp"));
    }

    #[test]
    fn external_host_is_empty_for_local_machine() {
        let mut spec = ServerSpec {
            id: ServerId::parse("neo").expect("id"),
            host: "localhost".into(),
            port: 22,
            user: "u".into(),
            auth: AuthMethod::Auto,
            jump: None,
            sudo: SudoMode::None,
            description: ServerDescription::default(),
            location: None,
            modules: Default::default(),
        };
        assert_eq!(spec.external_host(), "");
        spec.host = "neo.example".into();
        assert_eq!(spec.external_host(), "neo.example");
    }

    #[test]
    fn parse_empty_rejects_empty() {
        assert_eq!(ServerId::parse("   "), Err(InvalidServerName::Empty));
    }
}
