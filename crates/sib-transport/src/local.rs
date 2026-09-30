use std::process::Stdio;

use async_trait::async_trait;
use sib_core::{CommandOutput, SudoMode, Transport, TransportError};
use tokio::io::AsyncWriteExt;
use tokio::process::Command;

use crate::sudo;

pub struct LocalTransport {
    sudo_mode: SudoMode,
    sudo_password: Option<String>,
}

impl LocalTransport {
    pub fn new(sudo_mode: SudoMode, sudo_password: Option<String>) -> Self {
        Self {
            sudo_mode,
            sudo_password,
        }
    }

    async fn run(
        &self,
        command: &str,
        stdin: Option<&str>,
    ) -> Result<CommandOutput, TransportError> {
        let mut child = Command::new("sh")
            .arg("-c")
            .arg(command)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|e| TransportError::Exec(e.to_string()))?;
        if let (Some(input), Some(mut pipe)) = (stdin, child.stdin.take()) {
            pipe.write_all(input.as_bytes())
                .await
                .map_err(|e| TransportError::Exec(e.to_string()))?;
        }
        let output = child
            .wait_with_output()
            .await
            .map_err(|e| TransportError::Exec(e.to_string()))?;
        Ok(CommandOutput {
            stdout: String::from_utf8_lossy(&output.stdout).into_owned(),
            stderr: String::from_utf8_lossy(&output.stderr).into_owned(),
            exit_code: output.status.code().unwrap_or(-1),
        })
    }
}

#[async_trait]
impl Transport for LocalTransport {
    async fn exec(&self, command: &str) -> Result<CommandOutput, TransportError> {
        self.run(command, None).await
    }

    async fn exec_root(&self, command: &str) -> Result<CommandOutput, TransportError> {
        match self.sudo_mode {
            SudoMode::None => Err(TransportError::SudoUnavailable),
            SudoMode::Passwordless => self.run(&sudo::wrap_passwordless(command), None).await,
            SudoMode::WithPassword => {
                let password = self.sudo_password.as_deref().unwrap_or_default();
                let input = format!("{password}\n");
                self.run(&sudo::wrap_with_password(command), Some(&input))
                    .await
            }
        }
    }

    fn sudo_mode(&self) -> SudoMode {
        self.sudo_mode
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn exec_echo_returns_stdout() {
        let transport = LocalTransport::new(SudoMode::None, None);
        let output = transport.exec("echo hello").await.ok();
        assert_eq!(output.map(|o| o.stdout), Some("hello\n".to_owned()));
    }

    #[tokio::test]
    async fn exec_failing_command_returns_exit_code() {
        let transport = LocalTransport::new(SudoMode::None, None);
        let output = transport.exec("exit 3").await.ok();
        assert_eq!(output.map(|o| o.exit_code), Some(3));
    }

    #[tokio::test]
    async fn exec_root_without_sudo_is_unavailable() {
        let transport = LocalTransport::new(SudoMode::None, None);
        assert!(matches!(
            transport.exec_root("id").await,
            Err(TransportError::SudoUnavailable)
        ));
    }
}
