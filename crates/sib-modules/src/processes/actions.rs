use sib_core::{ActionOutcome, ActionRequest, ActionSpec, Danger, ModuleError, Transport};

use super::ID;
use crate::common::root::{exec_prefer_root, require_success};

pub const ACTION_TERMINATE: &str = "terminate";
pub const ACTION_KILL: &str = "kill";

pub const SPEC_TERMINATE: ActionSpec = ActionSpec::new(
    ID,
    ACTION_TERMINATE,
    "Stop the process (SIGTERM)",
    Danger::Normal,
);
pub const SPEC_KILL: ActionSpec =
    ActionSpec::new(ID, ACTION_KILL, "Kill the process (SIGKILL)", Danger::High);
pub const SPECS: [ActionSpec; 2] = [SPEC_TERMINATE, SPEC_KILL];

pub async fn perform(
    transport: &dyn Transport,
    request: &ActionRequest,
) -> Result<ActionOutcome, ModuleError> {
    let pid = validate_pid(&request.target)?;
    let signal = match request.kind.as_str() {
        ACTION_TERMINATE => "TERM",
        ACTION_KILL => "KILL",
        other => return Err(ModuleError::UnsupportedAction(other.to_owned())),
    };
    let command = format!("kill -{signal} {pid}");
    require_success(exec_prefer_root(transport, &command).await?)?;
    Ok(ActionOutcome::new(format!(
        "SIG{signal} sent to process {pid}"
    )))
}

fn validate_pid(value: &str) -> Result<u32, ModuleError> {
    value
        .trim()
        .parse::<u32>()
        .ok()
        .filter(|pid| *pid > 1)
        .ok_or_else(|| ModuleError::ActionFailed(format!("invalid PID {value}")))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn validate_pid_rejects_init_and_text() {
        assert_eq!(validate_pid(" 4242 ").expect("pid"), 4242);
        assert!(validate_pid("1").is_err());
        assert!(validate_pid("12 34").is_err());
    }
}
