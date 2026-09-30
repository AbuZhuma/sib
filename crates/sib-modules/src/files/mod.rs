mod actions;
mod model;
mod parse;
mod script;

use async_trait::async_trait;
use sib_core::{
    ActionOutcome, ActionRequest, ActionSpec, Availability, CollectContext, Module, ModuleError,
    ModuleId, ModuleSettings, QueryRequest, QueryResponse, Schedule, Snapshot, Transport,
};

pub use actions::{
    ACTION_CHMOD, ACTION_CHOWN, ACTION_COPY, ACTION_CREATE, ACTION_DELETE, ACTION_MKDIR,
    ACTION_MOVE, ACTION_WRITE, SPEC_CHMOD, SPEC_CHOWN, SPEC_COPY, SPEC_CREATE, SPEC_DELETE,
    SPEC_MKDIR, SPEC_MOVE, SPEC_WRITE, SPEC_WRITE_SYSTEM, write_spec,
};
pub use model::{Entry, EntryKind, Listing, join_path, parent_path};
pub use parse::listing as parse_listing;
pub use script::{MAX_ENTRIES, MAX_FILE_BYTES, MAX_SEARCH_RESULTS};

use crate::common::detect;
use crate::common::root::exec_prefer_root;

pub const ID: ModuleId = ModuleId("files");
pub const QUERY_LIST: &str = "list";
pub const QUERY_READ: &str = "read";
pub const QUERY_SEARCH: &str = "search";
const MAX_PATTERN_LEN: usize = 120;
pub const ROOT: &str = "/";

pub struct FilesModule;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FilesSnapshot;

#[async_trait]
impl Module for FilesModule {
    fn id(&self) -> ModuleId {
        ID
    }

    fn title(&self) -> &'static str {
        "Файлы"
    }

    fn schedule(&self) -> Schedule {
        Schedule::OnDemand
    }

    async fn detect(
        &self,
        transport: &dyn Transport,
        _settings: &ModuleSettings,
    ) -> Result<Availability, ModuleError> {
        detect::require(transport, script::DETECT, "нужны GNU find и stat").await
    }

    async fn collect(
        &self,
        _transport: &dyn Transport,
        _context: &CollectContext,
    ) -> Result<Snapshot, ModuleError> {
        Ok(Snapshot::new(FilesSnapshot))
    }

    async fn query(
        &self,
        transport: &dyn Transport,
        request: &QueryRequest,
    ) -> Result<QueryResponse, ModuleError> {
        match request.kind.as_str() {
            QUERY_LIST => list(transport, validate_path(&request.target)?).await,
            QUERY_READ => read(transport, validate_path(&request.target)?).await,
            QUERY_SEARCH => search(transport, validate_pattern(&request.target)?).await,
            other => Err(ModuleError::UnsupportedQuery(other.to_owned())),
        }
    }

    fn actions(&self) -> &'static [ActionSpec] {
        &actions::SPECS
    }

    async fn perform(
        &self,
        transport: &dyn Transport,
        request: &ActionRequest,
    ) -> Result<ActionOutcome, ModuleError> {
        actions::perform(transport, request).await
    }
}

async fn list(transport: &dyn Transport, path: &str) -> Result<QueryResponse, ModuleError> {
    let output = exec_prefer_root(transport, &script::list(path)).await?;
    if !output.is_success() && output.stdout.is_empty() {
        return Err(ModuleError::CommandFailed(failure_detail(&output.stderr)));
    }
    parse::listing(&output.stdout)?;
    Ok(QueryResponse {
        title: path.to_owned(),
        text: output.stdout,
    })
}

async fn read(transport: &dyn Transport, path: &str) -> Result<QueryResponse, ModuleError> {
    let output = exec_prefer_root(transport, &script::read(path)).await?;
    let reason = match output.exit_code {
        0 => {
            return Ok(QueryResponse {
                title: path.to_owned(),
                text: output.stdout,
            });
        }
        script::EXIT_UNREADABLE => "нет доступа на чтение".to_owned(),
        script::EXIT_TOO_LARGE => format!("файл больше {} КБ", MAX_FILE_BYTES / 1000),
        script::EXIT_BINARY => "бинарный файл".to_owned(),
        _ => failure_detail(&output.stderr),
    };
    Err(ModuleError::CommandFailed(format!("{path}: {reason}")))
}

async fn search(transport: &dyn Transport, pattern: &str) -> Result<QueryResponse, ModuleError> {
    let output = exec_prefer_root(transport, &script::search(pattern)).await?;
    parse::listing(&output.stdout)?;
    Ok(QueryResponse {
        title: pattern.to_owned(),
        text: output.stdout,
    })
}

pub fn validate_pattern(pattern: &str) -> Result<&str, ModuleError> {
    let trimmed = pattern.trim();
    let is_safe = !trimmed.is_empty()
        && trimmed.len() <= MAX_PATTERN_LEN
        && !trimmed.contains('/')
        && !trimmed.chars().any(char::is_control);
    if !is_safe {
        return Err(ModuleError::ActionFailed(format!(
            "шаблон поиска - имя без '/', до {MAX_PATTERN_LEN} символов: {pattern:?}"
        )));
    }
    Ok(trimmed)
}

fn failure_detail(stderr: &str) -> String {
    stderr
        .lines()
        .next()
        .unwrap_or("команда завершилась с ошибкой")
        .trim()
        .to_owned()
}

pub fn validate_path(path: &str) -> Result<&str, ModuleError> {
    let is_safe = path.starts_with('/')
        && !path.contains('\0')
        && !path.split('/').any(|segment| segment == "..");
    if !is_safe {
        return Err(ModuleError::ActionFailed(format!(
            "путь должен быть абсолютным без '..': {path:?}"
        )));
    }
    Ok(path)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn validate_path_requires_absolute_without_parent_segments() {
        assert!(validate_path("/etc/hosts").is_ok());
        assert!(validate_path("/").is_ok());
        assert!(validate_path("etc").is_err());
        assert!(validate_path("/etc/../shadow").is_err());
        assert!(validate_path("/a\0b").is_err());
    }

    #[test]
    fn validate_pattern_rejects_paths_and_empty() {
        assert_eq!(
            validate_pattern(" nginx.conf ").expect("pattern"),
            "nginx.conf"
        );
        assert!(validate_pattern("").is_err());
        assert!(validate_pattern("etc/passwd").is_err());
        assert!(validate_pattern("a\nb").is_err());
    }

    #[test]
    fn validate_path_keeps_shell_characters_for_quoting() {
        assert!(validate_path("/srv/it's here; rm").is_ok());
    }
}
