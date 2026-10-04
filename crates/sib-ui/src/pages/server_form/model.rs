use std::collections::BTreeMap;

use sib_core::{
    AuthMethod, CheckOverrides, Credentials, CustomCheck, Environment, JumpHost, ManualLocation,
    ModuleSettings, PipelineBinding, ServerDescription, ServerId, ServerSpec, SudoMode,
};

use crate::text;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum AuthChoice {
    #[default]
    Auto,
    KeyFile,
    Password,
}

#[derive(Debug, Default)]
pub struct FormFields {
    pub name: String,
    pub host: String,
    pub port: String,
    pub user: String,
    pub auth: AuthChoice,
    pub key_path: String,
    pub passphrase: String,
    pub password: String,
    pub jump_enabled: bool,
    pub jump_host: String,
    pub jump_port: String,
    pub jump_user: String,
    pub sudo: SudoMode,
    pub sudo_password: String,
    pub project: String,
    pub purpose: String,
    pub environment: Environment,
    pub tags: String,
    pub owner: String,
    pub links: String,
    pub notes: String,
    pub location: String,
    pub location_label: String,
    pub modules: BTreeMap<String, ModuleSettings>,
    pub checks: Vec<CustomCheck>,
    pub check_overrides: CheckOverrides,
    pub pipelines: Vec<PipelineBinding>,
}

impl FormFields {
    pub fn empty() -> Self {
        Self {
            port: "22".to_owned(),
            jump_port: "22".to_owned(),
            ..Self::default()
        }
    }

    pub fn from_spec(spec: &ServerSpec) -> Self {
        let (auth, key_path) = match &spec.auth {
            AuthMethod::Auto => (AuthChoice::Auto, String::new()),
            AuthMethod::KeyFile { path, .. } => (AuthChoice::KeyFile, path.display().to_string()),
            AuthMethod::Password => (AuthChoice::Password, String::new()),
        };
        let jump = spec.jump.clone().unwrap_or(JumpHost {
            host: String::new(),
            port: 22,
            user: String::new(),
        });
        Self {
            name: spec.id.to_string(),
            host: spec.host.clone(),
            port: spec.port.to_string(),
            user: spec.user.clone(),
            auth,
            key_path,
            jump_enabled: spec.jump.is_some(),
            jump_host: jump.host,
            jump_port: jump.port.to_string(),
            jump_user: jump.user,
            sudo: spec.sudo,
            location: spec
                .location
                .as_ref()
                .map(|l| format!("{}, {}", l.lat, l.lon))
                .unwrap_or_default(),
            location_label: spec
                .location
                .as_ref()
                .map(|l| l.label.clone())
                .unwrap_or_default(),
            modules: spec.modules.clone(),
            checks: spec.checks.clone(),
            check_overrides: spec.check_overrides.clone(),
            pipelines: spec.pipelines.clone(),
            ..Self::from_description(&spec.description)
        }
    }

    fn from_description(d: &ServerDescription) -> Self {
        Self {
            project: d.project.clone(),
            purpose: d.purpose.clone(),
            environment: d.environment,
            tags: d.tags.join(", "),
            owner: d.owner.clone(),
            links: d.links.join("\n"),
            notes: d.notes.clone(),
            ..Self::default()
        }
    }

    pub fn build(
        &self,
        existing: &[ServerId],
        editing: Option<&ServerId>,
    ) -> Result<(ServerSpec, Credentials), String> {
        let id = ServerId::parse(&self.name).map_err(|e| e.to_string())?;
        if editing != Some(&id) && existing.contains(&id) {
            return Err(text::ERR_NAME_TAKEN.to_owned());
        }
        let spec = ServerSpec {
            id,
            host: non_empty(&self.host, text::ERR_HOST_EMPTY)?,
            port: parse_port(&self.port)?,
            user: non_empty(&self.user, text::ERR_USER_EMPTY)?,
            auth: self.auth_method()?,
            jump: self.jump()?,
            sudo: self.sudo,
            description: self.description(),
            location: self.location()?,
            modules: self.modules.clone(),
            checks: self.checks.clone(),
            check_overrides: self.check_overrides.clone(),
            pipelines: self.pipelines.clone(),
        };
        Ok((spec, self.credentials()))
    }

    fn location(&self) -> Result<Option<ManualLocation>, String> {
        let raw = self.location.trim();
        if raw.is_empty() {
            return Ok(None);
        }
        let (lat, lon) = raw
            .split_once([',', ' '])
            .and_then(|(lat, lon)| Some((lat.trim().parse().ok()?, lon.trim().parse().ok()?)))
            .filter(|(lat, lon): &(f64, f64)| lat.abs() <= 90.0 && lon.abs() <= 180.0)
            .ok_or_else(|| text::ERR_LOCATION.to_owned())?;
        Ok(Some(ManualLocation {
            lat,
            lon,
            label: self.location_label.trim().to_owned(),
        }))
    }

    fn auth_method(&self) -> Result<AuthMethod, String> {
        Ok(match self.auth {
            AuthChoice::Auto => AuthMethod::Auto,
            AuthChoice::Password => AuthMethod::Password,
            AuthChoice::KeyFile => AuthMethod::KeyFile {
                path: non_empty(&self.key_path, text::ERR_KEY_PATH)?.into(),
                has_passphrase: !self.passphrase.is_empty(),
            },
        })
    }

    fn jump(&self) -> Result<Option<JumpHost>, String> {
        if !self.jump_enabled {
            return Ok(None);
        }
        if self.jump_host.trim().is_empty() || self.jump_user.trim().is_empty() {
            return Err(text::ERR_JUMP.to_owned());
        }
        Ok(Some(JumpHost {
            host: self.jump_host.trim().to_owned(),
            port: parse_port(&self.jump_port)?,
            user: self.jump_user.trim().to_owned(),
        }))
    }

    fn description(&self) -> ServerDescription {
        ServerDescription {
            project: self.project.trim().to_owned(),
            purpose: self.purpose.trim().to_owned(),
            notes: self.notes.trim().to_owned(),
            environment: self.environment,
            tags: split_list(&self.tags, ','),
            owner: self.owner.trim().to_owned(),
            links: split_list(&self.links, '\n'),
        }
    }

    fn credentials(&self) -> Credentials {
        Credentials {
            password: optional(&self.password),
            passphrase: optional(&self.passphrase),
            sudo_password: optional(&self.sudo_password),
        }
    }
}

fn non_empty(value: &str, error: &str) -> Result<String, String> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return Err(error.to_owned());
    }
    Ok(trimmed.to_owned())
}

fn parse_port(value: &str) -> Result<u16, String> {
    value
        .trim()
        .parse::<u16>()
        .ok()
        .filter(|p| *p > 0)
        .ok_or_else(|| text::ERR_PORT.to_owned())
}

fn optional(value: &str) -> Option<String> {
    (!value.is_empty()).then(|| value.to_owned())
}

fn split_list(value: &str, separator: char) -> Vec<String> {
    value
        .split(separator)
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(str::to_owned)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn filled() -> FormFields {
        FormFields {
            name: "neo".to_owned(),
            host: "1.2.3.4".to_owned(),
            user: "root".to_owned(),
            tags: "a, b,,c".to_owned(),
            ..FormFields::empty()
        }
    }

    #[test]
    fn build_valid_form_produces_spec() {
        let (spec, creds) = filled().build(&[], None).expect("valid");
        assert_eq!(spec.port, 22);
        assert_eq!(spec.description.tags, vec!["a", "b", "c"]);
        assert_eq!(creds, Credentials::default());
    }

    #[test]
    fn build_duplicate_name_is_error() {
        let existing = vec![ServerId::parse("neo").expect("id")];
        assert_eq!(
            filled().build(&existing, None),
            Err(text::ERR_NAME_TAKEN.to_owned())
        );
    }

    #[test]
    fn build_editing_same_name_is_allowed() {
        let id = ServerId::parse("neo").expect("id");
        assert!(filled().build(std::slice::from_ref(&id), Some(&id)).is_ok());
    }

    #[test]
    fn build_bad_port_is_error() {
        let mut form = filled();
        form.port = "99999".to_owned();
        assert_eq!(form.build(&[], None), Err(text::ERR_PORT.to_owned()));
    }

    #[test]
    fn from_spec_roundtrips_fields() {
        let (spec, _) = filled().build(&[], None).expect("valid");
        let again = FormFields::from_spec(&spec);
        assert_eq!(again.build(&[], Some(&spec.id)).map(|(s, _)| s), Ok(spec));
    }
}
