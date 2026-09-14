use asiba_core::{ActionOutcome, ActionRequest, ActionSpec, Danger, ModuleError, Transport};

use super::{BINARY, ID};
use crate::common::root::{require_success, validate_name};

pub const ACTION_RESTART: &str = "restart";
pub const ACTION_START: &str = "start";
pub const ACTION_STOP: &str = "stop";

pub const SPEC_RESTART: ActionSpec = ActionSpec::new(
    ID,
    ACTION_RESTART,
    "Перезапустить контейнер",
    Danger::Normal,
);
pub const SPEC_START: ActionSpec =
    ActionSpec::new(ID, ACTION_START, "Запустить контейнер", Danger::Normal);
pub const SPEC_STOP: ActionSpec =
    ActionSpec::new(ID, ACTION_STOP, "Остановить контейнер", Danger::High);
pub const SPECS: [ActionSpec; 3] = [SPEC_RESTART, SPEC_START, SPEC_STOP];

pub async fn perform(
    transport: &dyn Transport,
    request: &ActionRequest,
) -> Result<ActionOutcome, ModuleError> {
    let container = validate_name(&request.target)?;
    let verb = match request.kind.as_str() {
        ACTION_RESTART | ACTION_START | ACTION_STOP => request.kind.as_str(),
        other => return Err(ModuleError::UnsupportedAction(other.to_owned())),
    };
    let command = format!("{BINARY}; $D {verb} {container} 2>&1");
    require_success(transport.exec(&command).await?)?;
    Ok(ActionOutcome::new(format!("{verb} {container}: ok")))
}
