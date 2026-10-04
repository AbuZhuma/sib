use std::time::Duration;

use russh::ChannelMsg;
use russh::client::Handle;
use sib_core::transport::OutputSink;
use sib_core::{OutputChunk, OutputStream, TransportError};

use super::handler::ClientHandler;

const STDERR_STREAM: u32 = 1;

pub async fn run(
    session: &Handle<ClientHandler>,
    command: &str,
    stdin: Option<&str>,
    timeout: Option<Duration>,
    sink: OutputSink<'_>,
) -> Result<i32, TransportError> {
    if session.is_closed() {
        return Err(TransportError::Disconnected("session closed".to_owned()));
    }
    match timeout {
        Some(limit) => tokio::time::timeout(limit, run_inner(session, command, stdin, sink))
            .await
            .map_err(|_| TransportError::Timeout)?,
        None => run_inner(session, command, stdin, sink).await,
    }
}

async fn run_inner(
    session: &Handle<ClientHandler>,
    command: &str,
    stdin: Option<&str>,
    sink: OutputSink<'_>,
) -> Result<i32, TransportError> {
    let mut channel = session
        .channel_open_session()
        .await
        .map_err(|e| TransportError::Disconnected(e.to_string()))?;
    channel
        .exec(true, command)
        .await
        .map_err(|e| TransportError::Exec(e.to_string()))?;
    if let Some(input) = stdin {
        channel
            .data(input.as_bytes())
            .await
            .map_err(|e| TransportError::Exec(e.to_string()))?;
        channel
            .eof()
            .await
            .map_err(|e| TransportError::Exec(e.to_string()))?;
    }
    let mut exit_code = -1;
    while let Some(message) = channel.wait().await {
        match message {
            ChannelMsg::Data { data } => sink(OutputChunk {
                stream: OutputStream::Stdout,
                text: String::from_utf8_lossy(&data).into_owned(),
            }),
            ChannelMsg::ExtendedData { data, ext } if ext == STDERR_STREAM => sink(OutputChunk {
                stream: OutputStream::Stderr,
                text: String::from_utf8_lossy(&data).into_owned(),
            }),
            ChannelMsg::ExitStatus { exit_status } => exit_code = exit_status as i32,
            ChannelMsg::Close => break,
            _ => {}
        }
    }
    Ok(exit_code)
}
