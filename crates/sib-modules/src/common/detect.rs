use sib_core::{Availability, ModuleError, Transport};

pub async fn require(
    transport: &dyn Transport,
    probe: &str,
    reason: &str,
) -> Result<Availability, ModuleError> {
    let output = transport.exec(probe).await?;
    if !output.is_success() {
        return Ok(Availability::unavailable(reason));
    }
    Ok(Availability::Available)
}
