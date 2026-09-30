use async_trait::async_trait;

use crate::server::SudoMode;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CommandOutput {
    pub stdout: String,
    pub stderr: String,
    pub exit_code: i32,
}

impl CommandOutput {
    pub fn is_success(&self) -> bool {
        self.exit_code == 0
    }
}

#[derive(Debug, Clone, thiserror::Error)]
pub enum TransportError {
    #[error("не удалось подключиться: {0}")]
    Connect(String),
    #[error("ошибка аутентификации: {0}")]
    Auth(String),
    #[error("неизвестный ключ хоста: {fingerprint}")]
    UnknownHostKey { fingerprint: String },
    #[error("ключ хоста изменился: {fingerprint}")]
    HostKeyChanged { fingerprint: String },
    #[error("соединение потеряно: {0}")]
    Disconnected(String),
    #[error("ошибка выполнения команды: {0}")]
    Exec(String),
    #[error("sudo не настроен для этого сервера")]
    SudoUnavailable,
    #[error("таймаут операции")]
    Timeout,
}

#[async_trait]
pub trait Transport: Send + Sync {
    async fn exec(&self, command: &str) -> Result<CommandOutput, TransportError>;

    async fn exec_root(&self, command: &str) -> Result<CommandOutput, TransportError>;

    fn sudo_mode(&self) -> SudoMode;

    async fn read_file(&self, path: &str) -> Result<String, TransportError> {
        let output = self.exec(&format!("cat {}", shell_quote(path))).await?;
        if output.is_success() {
            return Ok(output.stdout);
        }
        Err(TransportError::Exec(output.stderr.trim().to_owned()))
    }
}

pub fn shell_quote(value: &str) -> String {
    format!("'{}'", value.replace('\'', "'\\''"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn shell_quote_plain_wraps_in_quotes() {
        assert_eq!(shell_quote("/etc/os-release"), "'/etc/os-release'");
    }

    #[test]
    fn shell_quote_single_quote_escapes() {
        assert_eq!(shell_quote("it's"), "'it'\\''s'");
    }
}
