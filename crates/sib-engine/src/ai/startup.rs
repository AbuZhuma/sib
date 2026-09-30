use std::time::Duration;

use sib_config::AiConfig;
use sib_core::{AuditScope, AuditTarget, SharedState};
use sib_modules::system;
use tokio::sync::{mpsc, watch};

use crate::command::Command;

const POLL: Duration = Duration::from_secs(5);
const MAX_WAIT: Duration = Duration::from_secs(120);

pub struct StartupSummary {
    pub state: SharedState,
    pub config: watch::Receiver<AiConfig>,
    pub commands: mpsc::UnboundedSender<Command>,
}

pub fn spawn(context: StartupSummary) {
    tokio::spawn(run(context));
}

async fn run(context: StartupSummary) {
    let started = tokio::time::Instant::now();
    loop {
        tokio::time::sleep(POLL).await;
        if all_servers_collected(&context.state) || started.elapsed() >= MAX_WAIT {
            break;
        }
    }
    if !context.config.borrow().is_ready() {
        return;
    }
    let _ = context.commands.send(Command::Audit {
        target: AuditTarget::Fleet,
        scope: AuditScope::Full,
        is_auto: true,
    });
}

fn all_servers_collected(state: &SharedState) -> bool {
    let Ok(state) = state.read() else {
        return false;
    };
    !state.servers.is_empty()
        && state
            .servers
            .values()
            .all(|s| s.snapshot(system::ID).is_some() || !s.connection.is_online())
}
