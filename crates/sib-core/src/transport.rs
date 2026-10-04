use std::time::Duration;

use async_trait::async_trait;

use crate::server::SudoMode;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OutputStream {
    Stdout,
    Stderr,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OutputChunk {
    pub stream: OutputStream,
    pub text: String,
}

pub type OutputSink<'a> = &'a (dyn Fn(OutputChunk) + Send + Sync);

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
    #[error("could not connect: {0}")]
    Connect(String),
    #[error("authentication failed: {0}")]
    Auth(String),
    #[error("unknown host key: {fingerprint}")]
    UnknownHostKey { fingerprint: String },
    #[error("host key changed: {fingerprint}")]
    HostKeyChanged { fingerprint: String },
    #[error("connection lost: {0}")]
    Disconnected(String),
    #[error("command execution failed: {0}")]
    Exec(String),
    #[error("sudo is not set up for this server")]
    SudoUnavailable,
    #[error("operation timed out")]
    Timeout,
}

#[async_trait]
pub trait Transport: Send + Sync {
    async fn exec(&self, command: &str) -> Result<CommandOutput, TransportError>;

    async fn exec_root(&self, command: &str) -> Result<CommandOutput, TransportError>;

    fn sudo_mode(&self) -> SudoMode;

    async fn exec_streaming(
        &self,
        command: &str,
        as_root: bool,
        timeout: Option<Duration>,
        sink: OutputSink<'_>,
    ) -> Result<i32, TransportError> {
        let _ = timeout;
        let output = if as_root {
            self.exec_root(command).await?
        } else {
            self.exec(command).await?
        };
        if !output.stdout.is_empty() {
            sink(OutputChunk {
                stream: OutputStream::Stdout,
                text: output.stdout,
            });
        }
        if !output.stderr.is_empty() {
            sink(OutputChunk {
                stream: OutputStream::Stderr,
                text: output.stderr,
            });
        }
        Ok(output.exit_code)
    }

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
