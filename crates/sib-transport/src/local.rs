use std::process::Stdio;
use std::time::Duration;

use async_trait::async_trait;
use sib_core::transport::OutputSink;
use sib_core::{CommandOutput, OutputChunk, OutputStream, SudoMode, Transport, TransportError};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
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

    fn root_command(&self, command: &str) -> Result<(String, Option<String>), TransportError> {
        match self.sudo_mode {
            SudoMode::None => Err(TransportError::SudoUnavailable),
            SudoMode::Passwordless => Ok((sudo::wrap_passwordless(command), None)),
            SudoMode::WithPassword => {
                let password = self.sudo_password.as_deref().unwrap_or_default();
                Ok((
                    sudo::wrap_with_password(command),
                    Some(format!("{password}\n")),
                ))
            }
        }
    }

    async fn run_streaming(
        command: &str,
        stdin: Option<&str>,
        sink: OutputSink<'_>,
    ) -> Result<i32, TransportError> {
        let mut child = Command::new("sh")
            .arg("-c")
            .arg(command)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .kill_on_drop(true)
            .spawn()
            .map_err(|e| TransportError::Exec(e.to_string()))?;
        if let Some(mut pipe) = child.stdin.take() {
            if let Some(input) = stdin {
                pipe.write_all(input.as_bytes())
                    .await
                    .map_err(|e| TransportError::Exec(e.to_string()))?;
            }
            drop(pipe);
        }
        let stdout = child.stdout.take();
        let stderr = child.stderr.take();
        let pump_out = pump(stdout, OutputStream::Stdout, sink);
        let pump_err = pump(stderr, OutputStream::Stderr, sink);
        let (_, _, status) = tokio::join!(pump_out, pump_err, child.wait());
        let status = status.map_err(|e| TransportError::Exec(e.to_string()))?;
        Ok(status.code().unwrap_or(-1))
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

async fn pump<R: AsyncReadExt + Unpin>(
    reader: Option<R>,
    stream: OutputStream,
    sink: OutputSink<'_>,
) {
    let Some(mut reader) = reader else { return };
    let mut buf = [0u8; 4096];
    loop {
        match reader.read(&mut buf).await {
            Ok(0) | Err(_) => break,
            Ok(n) => sink(OutputChunk {
                stream,
                text: String::from_utf8_lossy(&buf[..n]).into_owned(),
            }),
        }
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

    async fn exec_streaming(
        &self,
        command: &str,
        as_root: bool,
        timeout: Option<Duration>,
        sink: OutputSink<'_>,
    ) -> Result<i32, TransportError> {
        let (command, stdin) = if as_root {
            self.root_command(command)?
        } else {
            (command.to_owned(), None)
        };
        match timeout {
            Some(limit) => {
                tokio::time::timeout(limit, Self::run_streaming(&command, stdin.as_deref(), sink))
                    .await
                    .map_err(|_| TransportError::Timeout)?
            }
            None => Self::run_streaming(&command, stdin.as_deref(), sink).await,
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
    async fn exec_streaming_delivers_chunks_in_order_and_exit_code() {
        let transport = LocalTransport::new(SudoMode::None, None);
        let chunks = std::sync::Mutex::new(Vec::new());
        let sink = |c: OutputChunk| {
            if let Ok(mut v) = chunks.lock() {
                v.push(c);
            }
        };
        let code = transport
            .exec_streaming("echo one; echo two >&2; exit 4", false, None, &sink)
            .await
            .ok();
        assert_eq!(code, Some(4));
        let got = chunks.into_inner().unwrap_or_default();
        let out: String = got
            .iter()
            .filter(|c| c.stream == OutputStream::Stdout)
            .map(|c| c.text.as_str())
            .collect();
        let err: String = got
            .iter()
            .filter(|c| c.stream == OutputStream::Stderr)
            .map(|c| c.text.as_str())
            .collect();
        assert_eq!(out, "one\n");
        assert_eq!(err, "two\n");
    }

    #[tokio::test]
    async fn exec_streaming_times_out() {
        let transport = LocalTransport::new(SudoMode::None, None);
        let sink = |_: OutputChunk| {};
        let result = transport
            .exec_streaming("sleep 5", false, Some(Duration::from_millis(100)), &sink)
            .await;
        assert!(matches!(result, Err(TransportError::Timeout)));
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
