use asiba_core::{ActionOutcome, ActionRequest, ActionSpec, Danger, ModuleError, Transport};

use super::{ID, script, validate_path};
use crate::common::root::{exec_prefer_root, require_success, validate_name};

pub const ACTION_WRITE: &str = "write";
pub const ACTION_CHMOD: &str = "chmod";
pub const ACTION_CHOWN: &str = "chown";
pub const ACTION_DELETE: &str = "delete";
pub const ACTION_MKDIR: &str = "mkdir";
pub const ACTION_CREATE: &str = "create";
pub const ACTION_MOVE: &str = "move";
pub const ACTION_COPY: &str = "copy";

pub const SPEC_WRITE: ActionSpec =
    ActionSpec::new(ID, ACTION_WRITE, "Сохранить файл", Danger::Normal);
pub const SPEC_WRITE_SYSTEM: ActionSpec =
    ActionSpec::new(ID, ACTION_WRITE, "Сохранить системный файл", Danger::High);
const SYSTEM_PREFIXES: [&str; 6] = ["/etc/", "/boot/", "/usr/", "/bin/", "/sbin/", "/lib/"];

pub fn write_spec(path: &str) -> ActionSpec {
    if SYSTEM_PREFIXES
        .iter()
        .any(|prefix| path.starts_with(prefix))
    {
        return SPEC_WRITE_SYSTEM;
    }
    SPEC_WRITE
}
pub const SPEC_CHMOD: ActionSpec =
    ActionSpec::new(ID, ACTION_CHMOD, "Изменить права", Danger::Normal);
pub const SPEC_CHOWN: ActionSpec =
    ActionSpec::new(ID, ACTION_CHOWN, "Изменить владельца", Danger::Normal);
pub const SPEC_DELETE: ActionSpec = ActionSpec::new(ID, ACTION_DELETE, "Удалить", Danger::High);
pub const SPEC_MKDIR: ActionSpec =
    ActionSpec::new(ID, ACTION_MKDIR, "Создать папку", Danger::Normal);
pub const SPEC_CREATE: ActionSpec =
    ActionSpec::new(ID, ACTION_CREATE, "Создать файл", Danger::Normal);
pub const SPEC_MOVE: ActionSpec = ActionSpec::new(
    ID,
    ACTION_MOVE,
    "Переместить или переименовать",
    Danger::Normal,
);
pub const SPEC_COPY: ActionSpec = ActionSpec::new(ID, ACTION_COPY, "Копировать", Danger::Normal);
pub const SPECS: [ActionSpec; 8] = [
    SPEC_WRITE,
    SPEC_CHMOD,
    SPEC_CHOWN,
    SPEC_DELETE,
    SPEC_MKDIR,
    SPEC_CREATE,
    SPEC_MOVE,
    SPEC_COPY,
];

const MODE_DIGITS: std::ops::RangeInclusive<usize> = 3..=4;
const ROOT: &str = "/";

pub async fn perform(
    transport: &dyn Transport,
    request: &ActionRequest,
) -> Result<ActionOutcome, ModuleError> {
    let path = validate_path(&request.target)?;
    let argument = request.argument.as_deref().unwrap_or_default();
    let command = match request.kind.as_str() {
        ACTION_WRITE => script::write(path, argument),
        ACTION_CHMOD => script::chmod(path, validate_mode(argument)?),
        ACTION_CHOWN => script::chown(path, &validate_name(argument)?),
        ACTION_DELETE => script::delete(validate_deletable(path)?),
        ACTION_MKDIR => script::make_directory(path),
        ACTION_CREATE => script::create_file(path),
        ACTION_MOVE => script::move_path(validate_deletable(path)?, validate_path(argument)?),
        ACTION_COPY => script::copy_path(path, validate_path(argument)?),
        other => return Err(ModuleError::UnsupportedAction(other.to_owned())),
    };
    require_success(exec_prefer_root(transport, &command).await?)?;
    Ok(ActionOutcome::new(outcome_message(
        &request.kind,
        path,
        argument,
    )))
}

fn outcome_message(kind: &str, path: &str, argument: &str) -> String {
    match kind {
        ACTION_WRITE => format!("{path}: записано {} байт", argument.len()),
        ACTION_DELETE => format!("{path}: удалено"),
        ACTION_MKDIR | ACTION_CREATE => format!("{path}: создано"),
        ACTION_MOVE | ACTION_COPY => format!("{path} → {argument}: ok"),
        _ => format!("{kind} {argument} {path}: ok"),
    }
}

fn validate_mode(value: &str) -> Result<&str, ModuleError> {
    let trimmed = value.trim();
    let is_octal =
        MODE_DIGITS.contains(&trimmed.len()) && trimmed.chars().all(|c| ('0'..='7').contains(&c));
    if !is_octal {
        return Err(ModuleError::ActionFailed(format!(
            "права должны быть восьмеричным числом, получено {trimmed:?}"
        )));
    }
    Ok(trimmed)
}

fn validate_deletable(path: &str) -> Result<&str, ModuleError> {
    if path.trim_end_matches('/').is_empty() || path == ROOT {
        return Err(ModuleError::ActionFailed(
            "корень удалять нельзя".to_owned(),
        ));
    }
    Ok(path)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mode_accepts_three_and_four_octal_digits() {
        assert_eq!(validate_mode(" 644 ").expect("mode"), "644");
        assert_eq!(validate_mode("1777").expect("mode"), "1777");
        assert!(validate_mode("8").is_err());
        assert!(validate_mode("u+x").is_err());
        assert!(validate_mode("64").is_err());
    }

    #[test]
    fn write_spec_is_dangerous_under_system_directories() {
        assert_eq!(write_spec("/etc/fstab").danger, Danger::High);
        assert_eq!(write_spec("/srv/app/.env").danger, Danger::Normal);
    }

    #[test]
    fn root_is_not_deletable() {
        assert!(validate_deletable("/").is_err());
        assert!(validate_deletable("//").is_err());
        assert_eq!(validate_deletable("/tmp/x").expect("path"), "/tmp/x");
    }
}
