mod auth;
mod command;
mod handler;
mod session;

use asiba_core::{CommandOutput, Credentials, ServerSpec, SudoMode, Transport, TransportError};
use async_trait::async_trait;
use russh::client::Handle;

use crate::connect::HostKeyPolicy;
use crate::sudo;
use handler::ClientHandler;

pub struct SshTransport {
    handle: Handle<ClientHandler>,
    _jump: Option<Handle<ClientHandler>>,
    sudo_mode: SudoMode,
    sudo_password: Option<String>,
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
        })
    }

    pub fn is_alive(&self) -> bool {
        !self.handle.is_closed()
    }
}

#[async_trait]
impl Transport for SshTransport {
    async fn exec(&self, command: &str) -> Result<CommandOutput, TransportError> {
        command::run(&self.handle, command, None).await
    }

    async fn exec_root(&self, command: &str) -> Result<CommandOutput, TransportError> {
        match self.sudo_mode {
            SudoMode::None => Err(TransportError::SudoUnavailable),
            SudoMode::Passwordless => {
                command::run(&self.handle, &sudo::wrap_passwordless(command), None).await
            }
            SudoMode::WithPassword => {
                let password = self.sudo_password.as_deref().unwrap_or_default();
                let input = format!("{password}\n");
                command::run(
                    &self.handle,
                    &sudo::wrap_with_password(command),
                    Some(&input),
                )
                .await
            }
        }
    }

    fn sudo_mode(&self) -> SudoMode {
        self.sudo_mode
    }
}
