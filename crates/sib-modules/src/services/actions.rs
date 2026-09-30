use sib_core::{ActionOutcome, ActionRequest, ActionSpec, Danger, ModuleError, Transport};

use super::ID;
use crate::common::root::{exec_as_root, require_success, validate_name};

pub const ACTION_RESTART: &str = "restart";
pub const ACTION_START: &str = "start";
pub const ACTION_STOP: &str = "stop";

pub const SPEC_RESTART: ActionSpec =
    ActionSpec::new(ID, ACTION_RESTART, "Перезапустить сервис", Danger::Normal);
pub const SPEC_START: ActionSpec =
    ActionSpec::new(ID, ACTION_START, "Запустить сервис", Danger::Normal);
pub const SPEC_STOP: ActionSpec =
    ActionSpec::new(ID, ACTION_STOP, "Остановить сервис", Danger::High);
pub const SPECS: [ActionSpec; 3] = [SPEC_RESTART, SPEC_START, SPEC_STOP];

pub async fn perform(
    transport: &dyn Transport,
    request: &ActionRequest,
) -> Result<ActionOutcome, ModuleError> {
    let unit = validate_name(&request.target)?;
    let verb = match request.kind.as_str() {
        ACTION_RESTART | ACTION_START | ACTION_STOP => request.kind.as_str(),
        other => return Err(ModuleError::UnsupportedAction(other.to_owned())),
    };
    let command = action_command(verb, &unit);
    let state = require_success(exec_as_root(transport, &command).await?)?;
    Ok(ActionOutcome::new(format!(
        "systemctl {verb} {unit}: {state}"
    )))
}

fn action_command(verb: &str, unit: &str) -> String {
    format!("systemctl {verb} {unit} && {{ systemctl is-active {unit}; true; }}")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn action_command_reports_state_but_fails_only_on_the_action_itself() {
        let command = action_command(ACTION_STOP, "nginx.service");
        assert!(command.starts_with("systemctl stop nginx.service && {"));
        assert!(command.ends_with("systemctl is-active nginx.service; true; }"));
    }
}
