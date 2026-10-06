use std::collections::BTreeMap;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::server::{AuthMethod, ServerId, ServerSpec};
use crate::transport::shell_quote;

pub const DEFAULT_STEP_TIMEOUT_SECS: u64 = 3600;
pub const MAX_STEP_OUTPUT: usize = 200_000;
const MAX_ID_LEN: usize = 48;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum StepTarget {
    #[default]
    Server,
    Local,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Step {
    pub name: String,
    pub command: String,
    #[serde(default)]
    pub target: StepTarget,
    #[serde(default)]
    pub as_root: bool,
    #[serde(default = "default_timeout")]
    pub timeout_secs: u64,
    #[serde(default)]
    pub continue_on_error: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub rollback: Option<String>,
}

impl Step {
    pub fn rollback_command(&self) -> Option<&str> {
        self.rollback
            .as_deref()
            .map(str::trim)
            .filter(|s| !s.is_empty())
    }
}

fn default_timeout() -> u64 {
    DEFAULT_STEP_TIMEOUT_SECS
}

impl Default for Step {
    fn default() -> Self {
        Self {
            name: String::new(),
            command: String::new(),
            target: StepTarget::Server,
            as_root: false,
            timeout_secs: DEFAULT_STEP_TIMEOUT_SECS,
            continue_on_error: false,
            rollback: None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct Variable {
    pub name: String,
    #[serde(default)]
    pub default: String,
    #[serde(default)]
    pub secret: bool,
    #[serde(default)]
    pub description: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct Pipeline {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub description: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub variables: Vec<Variable>,
    #[serde(default)]
    pub steps: Vec<Step>,
}

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum InvalidPipeline {
    #[error("id cannot be empty")]
    EmptyId,
    #[error("id is longer than {MAX_ID_LEN} characters")]
    IdTooLong,
    #[error("id may contain only lowercase latin letters, digits, hyphens and underscores")]
    BadIdChar,
    #[error("name cannot be empty")]
    EmptyName,
    #[error("pipeline has no steps")]
    NoSteps,
    #[error("step {0} has no name")]
    StepWithoutName(usize),
    #[error("step '{0}' has no command")]
    StepWithoutCommand(String),
    #[error("variable name '{0}' is not a valid identifier")]
    BadVariableName(String),
    #[error("variable '{0}' is declared twice")]
    DuplicateVariable(String),
}

impl Pipeline {
    pub fn validate(&self) -> Result<(), InvalidPipeline> {
        validate_id(&self.id)?;
        if self.name.trim().is_empty() {
            return Err(InvalidPipeline::EmptyName);
        }
        if self.steps.is_empty() {
            return Err(InvalidPipeline::NoSteps);
        }
        for (index, step) in self.steps.iter().enumerate() {
            if step.name.trim().is_empty() {
                return Err(InvalidPipeline::StepWithoutName(index + 1));
            }
            if step.command.trim().is_empty() {
                return Err(InvalidPipeline::StepWithoutCommand(step.name.clone()));
            }
        }
        let mut seen = Vec::new();
        for variable in &self.variables {
            if !is_identifier(&variable.name) {
                return Err(InvalidPipeline::BadVariableName(variable.name.clone()));
            }
            if seen.contains(&variable.name.as_str()) {
                return Err(InvalidPipeline::DuplicateVariable(variable.name.clone()));
            }
            seen.push(variable.name.as_str());
        }
        Ok(())
    }

    pub fn variable(&self, name: &str) -> Option<&Variable> {
        self.variables.iter().find(|v| v.name == name)
    }
}

pub fn validate_id(raw: &str) -> Result<(), InvalidPipeline> {
    if raw.is_empty() {
        return Err(InvalidPipeline::EmptyId);
    }
    if raw.len() > MAX_ID_LEN {
        return Err(InvalidPipeline::IdTooLong);
    }
    let allowed = |c: char| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-' || c == '_';
    if !raw.chars().all(allowed) {
        return Err(InvalidPipeline::BadIdChar);
    }
    Ok(())
}

pub fn slug(raw: &str) -> String {
    let mut out = String::new();
    let mut last_dash = true;
    for c in raw.trim().chars() {
        let c = c.to_ascii_lowercase();
        if c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_' {
            out.push(c);
            last_dash = false;
        } else if !last_dash {
            out.push('-');
            last_dash = true;
        }
    }
    while out.ends_with('-') {
        out.pop();
    }
    out.chars().take(MAX_ID_LEN).collect()
}

fn is_identifier(name: &str) -> bool {
    let mut chars = name.chars();
    matches!(chars.next(), Some(c) if c.is_ascii_alphabetic() || c == '_')
        && chars.all(|c| c.is_ascii_alphanumeric() || c == '_')
}

#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct PipelineBinding {
    pub pipeline: String,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub values: BTreeMap<String, String>,
}

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum RenderError {
    #[error("unknown variable '{0}'")]
    UnknownVariable(String),
    #[error("unclosed '{{{{' in template")]
    Unclosed,
}

pub fn builtin_values(spec: &ServerSpec) -> BTreeMap<String, String> {
    let mut values = BTreeMap::new();
    values.insert("server".to_owned(), spec.id.to_string());
    values.insert("host".to_owned(), spec.host.clone());
    values.insert("user".to_owned(), spec.user.clone());
    values.insert("port".to_owned(), spec.port.to_string());
    values.insert(
        "ssh_target".to_owned(),
        format!("{}@{}", spec.user, spec.host),
    );
    values.insert("ssh_args".to_owned(), ssh_options(spec).join(" "));
    values
}

pub fn ssh_options(spec: &ServerSpec) -> Vec<String> {
    let mut args = vec!["-p".to_owned(), spec.port.to_string()];
    if let Some(jump) = &spec.jump {
        args.push("-J".to_owned());
        args.push(format!("{}@{}:{}", jump.user, jump.host, jump.port));
    }
    if let AuthMethod::KeyFile { path, .. } = &spec.auth {
        args.push("-i".to_owned());
        args.push(path.display().to_string());
    }
    args
}

pub fn render(template: &str, values: &BTreeMap<String, String>) -> Result<String, RenderError> {
    let mut out = String::with_capacity(template.len());
    let mut rest = template;
    while let Some(start) = rest.find("{{") {
        out.push_str(&rest[..start]);
        let after = &rest[start + 2..];
        let end = after.find("}}").ok_or(RenderError::Unclosed)?;
        let key = after[..end].trim();
        let (raw, name) = match key.strip_prefix("raw:") {
            Some(name) => (true, name.trim()),
            None => (false, key),
        };
        let value = values
            .get(name)
            .ok_or_else(|| RenderError::UnknownVariable(name.to_owned()))?;
        if raw || name == "ssh_args" {
            out.push_str(value);
        } else {
            out.push_str(&shell_quote(value));
        }
        rest = &after[end + 2..];
    }
    out.push_str(rest);
    Ok(out)
}

pub fn placeholders(template: &str) -> Vec<String> {
    let mut names = Vec::new();
    let mut rest = template;
    while let Some(start) = rest.find("{{") {
        let after = &rest[start + 2..];
        let Some(end) = after.find("}}") else { break };
        let key = after[..end].trim();
        let name = key.strip_prefix("raw:").map_or(key, str::trim).to_owned();
        if !names.contains(&name) {
            names.push(name);
        }
        rest = &after[end + 2..];
    }
    names
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RunStatus {
    Running,
    Done,
    Failed(String),
    Cancelled,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum StepStatus {
    Pending,
    Running,
    Done,
    Failed,
    Skipped,
    RolledBack,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StepRun {
    pub name: String,
    pub target: StepTarget,
    pub status: StepStatus,
    pub exit_code: Option<i32>,
    pub output: String,
    pub started_at: Option<DateTime<Utc>>,
    pub finished_at: Option<DateTime<Utc>>,
}

impl StepRun {
    pub fn pending(step: &Step) -> Self {
        Self {
            name: step.name.clone(),
            target: step.target,
            status: StepStatus::Pending,
            exit_code: None,
            output: String::new(),
            started_at: None,
            finished_at: None,
        }
    }

    pub fn append_output(&mut self, text: &str) {
        self.output.push_str(text);
        if self.output.len() > MAX_STEP_OUTPUT {
            let cut = self.output.len() - MAX_STEP_OUTPUT;
            let boundary = (cut..self.output.len())
                .find(|&i| self.output.is_char_boundary(i))
                .unwrap_or(self.output.len());
            self.output.drain(..boundary);
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PipelineRun {
    pub id: u64,
    pub server: ServerId,
    pub pipeline: String,
    pub pipeline_name: String,
    pub started_at: DateTime<Utc>,
    pub finished_at: Option<DateTime<Utc>>,
    pub status: RunStatus,
    pub steps: Vec<StepRun>,
}

impl PipelineRun {
    pub fn is_running(&self) -> bool {
        self.status == RunStatus::Running
    }

    pub fn is_success(&self) -> bool {
        self.status == RunStatus::Done
    }

    pub fn current_step(&self) -> Option<usize> {
        self.steps
            .iter()
            .position(|s| s.status == StepStatus::Running)
    }

    pub fn summary(&self) -> String {
        let done = self
            .steps
            .iter()
            .filter(|s| s.status == StepStatus::Done)
            .count();
        match &self.status {
            RunStatus::Running => format!("running, {done}/{} steps done", self.steps.len()),
            RunStatus::Done => format!("done, {} steps", self.steps.len()),
            RunStatus::Failed(reason) => format!("failed after {done} steps: {reason}"),
            RunStatus::Cancelled => format!("cancelled after {done} steps"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn values() -> BTreeMap<String, String> {
        let mut v = BTreeMap::new();
        v.insert("dir".to_owned(), "/srv/it's".to_owned());
        v.insert("ssh_args".to_owned(), "-p 22 -J a@b:22".to_owned());
        v.insert("threads".to_owned(), "16".to_owned());
        v
    }

    #[test]
    fn render_quotes_values_and_keeps_raw_ones() {
        let out = render(
            "cd {{dir}} && rsync -e \"ssh {{ssh_args}}\" --threads {{ raw:threads }}",
            &values(),
        );
        assert_eq!(
            out,
            Ok("cd '/srv/it'\\''s' && rsync -e \"ssh -p 22 -J a@b:22\" --threads 16".to_owned())
        );
    }

    #[test]
    fn render_reports_unknown_and_unclosed() {
        assert_eq!(
            render("echo {{nope}}", &values()),
            Err(RenderError::UnknownVariable("nope".to_owned()))
        );
        assert_eq!(render("echo {{dir", &values()), Err(RenderError::Unclosed));
    }

    #[test]
    fn placeholders_lists_each_name_once() {
        assert_eq!(
            placeholders("{{a}} {{raw:b}} {{a}}"),
            vec!["a".to_owned(), "b".to_owned()]
        );
    }

    #[test]
    fn validate_catches_missing_pieces() {
        let mut p = Pipeline {
            id: "deploy".into(),
            name: "Deploy".into(),
            ..Default::default()
        };
        assert_eq!(p.validate(), Err(InvalidPipeline::NoSteps));
        p.steps.push(Step {
            name: "build".into(),
            command: String::new(),
            ..Default::default()
        });
        assert_eq!(
            p.validate(),
            Err(InvalidPipeline::StepWithoutCommand("build".into()))
        );
        p.steps[0].command = "cargo build".into();
        p.variables.push(Variable {
            name: "1bad".into(),
            ..Default::default()
        });
        assert_eq!(
            p.validate(),
            Err(InvalidPipeline::BadVariableName("1bad".into()))
        );
        p.variables[0].name = "ok".into();
        assert_eq!(p.validate(), Ok(()));
    }

    #[test]
    fn slug_normalises_names() {
        assert_eq!(slug("  Brain Evolve v2! "), "brain-evolve-v2");
        assert_eq!(validate_id(&slug("Brain Evolve v2!")), Ok(()));
    }

    #[test]
    fn step_output_is_capped_at_char_boundary() {
        let mut run = StepRun::pending(&Step::default());
        run.append_output(&"é".repeat(MAX_STEP_OUTPUT));
        run.append_output("tail");
        assert!(run.output.len() <= MAX_STEP_OUTPUT);
        assert!(run.output.ends_with("tail"));
    }
}
