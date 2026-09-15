use std::time::Duration;

use asiba_core::{CommandOutput, TransportError};
use russh::ChannelMsg;
use russh::client::Handle;

use super::handler::ClientHandler;

const COMMAND_TIMEOUT: Duration = Duration::from_secs(60);
const STDERR_STREAM: u32 = 1;

pub async fn run(
    session: &Handle<ClientHandler>,
    command: &str,
    stdin: Option<&str>,
) -> Result<CommandOutput, TransportError> {
    if session.is_closed() {
        return Err(TransportError::Disconnected("сессия закрыта".to_owned()));
    }
    tokio::time::timeout(COMMAND_TIMEOUT, run_inner(session, command, stdin))
        .await
        .map_err(|_| TransportError::Timeout)?
}

async fn run_inner(
    session: &Handle<ClientHandler>,
    command: &str,
    stdin: Option<&str>,
) -> Result<CommandOutput, TransportError> {
    let mut channel = session.channel_open_session().await.map_err(disconnected)?;
    channel.exec(true, command).await.map_err(exec_failed)?;
    if let Some(input) = stdin {
        channel.data(input.as_bytes()).await.map_err(exec_failed)?;
        channel.eof().await.map_err(exec_failed)?;
    }
    let mut stdout = Vec::new();
    let mut stderr = Vec::new();
    let mut exit_code = -1;
    while let Some(message) = channel.wait().await {
        match message {
            ChannelMsg::Data { data } => stdout.extend_from_slice(&data),
            ChannelMsg::ExtendedData { data, ext } if ext == STDERR_STREAM => {
                stderr.extend_from_slice(&data);
            }
            ChannelMsg::ExitStatus { exit_status } => exit_code = exit_status as i32,
            ChannelMsg::Close => break,
            _ => {}
        }
    }
    Ok(CommandOutput {
        stdout: String::from_utf8_lossy(&stdout).into_owned(),
        stderr: String::from_utf8_lossy(&stderr).into_owned(),
        exit_code,
    })
}

fn disconnected(error: russh::Error) -> TransportError {
    match error {
        russh::Error::ChannelOpenFailure(reason) => {
            TransportError::Exec(format!("канал не открыт: {reason:?}"))
        }
        other => TransportError::Disconnected(other.to_string()),
    }
}

fn exec_failed(error: russh::Error) -> TransportError {
    TransportError::Exec(error.to_string())
}
