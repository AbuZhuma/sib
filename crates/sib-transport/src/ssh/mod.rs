mod auth;
mod command;
mod handler;
mod session;
mod ssh_config;
mod stream;

pub use ssh_config::effective_address;

use std::time::Duration;

use async_trait::async_trait;
use russh::client::Handle;
use sib_core::transport::OutputSink;
use sib_core::{CommandOutput, Credentials, ServerSpec, SudoMode, Transport, TransportError};
use tokio::sync::Semaphore;

use crate::connect::HostKeyPolicy;
use crate::sudo;
use handler::ClientHandler;

const MAX_CONCURRENT_CHANNELS: usize = 4;

pub struct SshTransport {
    handle: Handle<ClientHandler>,
    _jump: Option<Handle<ClientHandler>>,
    sudo_mode: SudoMode,
    sudo_password: Option<String>,
    channels: Semaphore,
}

impl SshTransport {
    pub async fn connect(
        spec: &ServerSpec,
        credentials: &Credentials,
        policy: HostKeyPolicy,
    ) -> Result<Self, TransportError> {
        let (handle, jump) = session::establish(spec, credentials, policy).await?;
        Ok(Self {
            handle,
            _jump: jump,
            sudo_mode: spec.sudo,
            sudo_password: credentials.sudo_password.clone(),
            channels: Semaphore::new(MAX_CONCURRENT_CHANNELS),
        })
    }

    async fn run(
        &self,
        command: &str,
        stdin: Option<&str>,
    ) -> Result<CommandOutput, TransportError> {
        let _slot = self
            .channels
            .acquire()
            .await
            .map_err(|_| TransportError::Disconnected("transport closed".to_owned()))?;
        command::run(&self.handle, command, stdin).await
    }

    pub fn is_alive(&self) -> bool {
        !self.handle.is_closed()
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
}

#[async_trait]
impl Transport for SshTransport {
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
        let _slot = self
            .channels
            .acquire()
            .await
            .map_err(|_| TransportError::Disconnected("transport closed".to_owned()))?;
        stream::run(&self.handle, &command, stdin.as_deref(), timeout, sink).await
    }

    fn sudo_mode(&self) -> SudoMode {
        self.sudo_mode
    }
}
