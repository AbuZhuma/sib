use sib_core::transport::shell_quote;
use sib_core::{CommandOutput, ModuleError, SudoMode, Transport};

pub async fn exec_prefer_root(
    transport: &dyn Transport,
    command: &str,
) -> Result<CommandOutput, ModuleError> {
    match transport.sudo_mode() {
        SudoMode::None => Ok(transport.exec(command).await?),
        _ => match transport.exec_root(command).await {
            Ok(output) if output.is_success() => Ok(output),
            _ => Ok(transport.exec(command).await?),
        },
    }
}

pub async fn exec_as_root(
    transport: &dyn Transport,
    command: &str,
) -> Result<CommandOutput, ModuleError> {
    match transport.sudo_mode() {
        SudoMode::None => {
            let whoami = transport.exec("id -un").await?;
            if whoami.stdout.trim() == "root" {
                return Ok(transport.exec(command).await?);
            }
            Ok(transport.exec_root(command).await?)
        }
        _ => Ok(transport.exec_root(command).await?),
    }
}

pub fn require_success(output: CommandOutput) -> Result<String, ModuleError> {
    if output.is_success() {
        return Ok(output.stdout.trim().to_owned());
    }
    let detail = if output.stderr.trim().is_empty() {
        output.stdout.trim().to_owned()
    } else {
        output.stderr.trim().to_owned()
    };
    Err(ModuleError::ActionFailed(format!(
        "код {}: {detail}",
        output.exit_code
    )))
}

pub fn validate_ip(value: &str) -> Result<String, ModuleError> {
    let trimmed = value.trim();
    trimmed
        .parse::<std::net::IpAddr>()
        .map(|ip| ip.to_string())
        .map_err(|_| ModuleError::ActionFailed(format!("некорректный IP {}", shell_quote(trimmed))))
}

pub fn validate_name(value: &str) -> Result<String, ModuleError> {
    let trimmed = value.trim();
    let is_safe = !trimmed.is_empty()
        && trimmed
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '-' | '_' | '.' | '@' | ':'));
    if !is_safe {
        return Err(ModuleError::ActionFailed(format!(
            "недопустимое имя {}",
            shell_quote(trimmed)
        )));
    }
    Ok(trimmed.to_owned())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn validate_ip_accepts_v4_and_v6() {
        assert_eq!(validate_ip(" 1.2.3.4 ").expect("ip"), "1.2.3.4");
        assert_eq!(validate_ip("::1").expect("ip"), "::1");
    }

    #[test]
    fn validate_ip_rejects_shell_text() {
        assert!(validate_ip("1.2.3.4; rm -rf /").is_err());
    }

    #[test]
    fn validate_name_rejects_spaces_and_quotes() {
        assert!(validate_name("nginx.service").is_ok());
        assert!(validate_name("a b").is_err());
        assert!(validate_name("x'y").is_err());
    }
}
