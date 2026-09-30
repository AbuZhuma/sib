use std::path::PathBuf;
use std::sync::Arc;

use chrono::Utc;
use sib_core::{ActionRecord, ActionRequest, Module, ServerId, SharedState, Transport};
use sib_modules::files;
use sib_storage::{HistoryReader, StorageWriter};
use tokio::sync::mpsc;

use crate::command::EngineEvent;
use crate::engine::RepaintNotifier;

const JOURNAL_PREFILL: usize = 200;
const MAX_JOURNALED_ARGUMENT: usize = 120;

pub struct Perform {
    pub server: ServerId,
    pub module: Option<Arc<dyn Module>>,
    pub transport: Option<Arc<dyn Transport>>,
    pub request: ActionRequest,
    pub state: SharedState,
    pub storage: Option<StorageWriter>,
    pub events: mpsc::UnboundedSender<EngineEvent>,
    pub notify: RepaintNotifier,
}

pub fn perform(job: Perform) {
    tokio::spawn(async move {
        let module_name = job
            .module
            .as_ref()
            .map(|m| m.id().to_string())
            .unwrap_or_default();
        let result = execute(&job).await;
        let argument = job
            .request
            .argument
            .map(|argument| journaled_argument(&module_name, &job.request.kind, argument));
        let record = ActionRecord {
            at: Utc::now(),
            server: job.server,
            module: module_name,
            kind: job.request.kind,
            target: job.request.target,
            argument,
            is_success: result.is_ok(),
            message: result.unwrap_or_else(|error| error),
        };
        log_record(&record);
        if let Some(storage) = &job.storage {
            let _ = storage.write_action(record.clone());
        }
        if let Ok(mut state) = job.state.write() {
            state.push_action(record.clone());
        }
        let _ = job.events.send(EngineEvent::ActionFinished(record));
        (job.notify)();
    });
}

async fn execute(job: &Perform) -> Result<String, String> {
    match (&job.transport, &job.module) {
        (Some(transport), Some(module)) => module
            .perform(transport.as_ref(), &job.request)
            .await
            .map(|outcome| outcome.message)
            .map_err(|e| e.to_string()),
        (None, _) => Err("server is not connected".to_owned()),
        (_, None) => Err("module not found".to_owned()),
    }
}

fn journaled_argument(module: &str, kind: &str, argument: String) -> String {
    let is_file_content = module == files::ID.0 && kind == files::ACTION_WRITE;
    if !is_file_content && argument.chars().count() <= MAX_JOURNALED_ARGUMENT {
        return argument;
    }
    format!("{} bytes", argument.len())
}

fn log_record(record: &ActionRecord) {
    if record.is_success {
        tracing::info!(server = %record.server, kind = %record.kind, target = %record.target, "action done");
    } else {
        tracing::warn!(server = %record.server, kind = %record.kind, target = %record.target, error = %record.message, "action failed");
    }
}

pub fn prefill_journal(path: Option<PathBuf>, state: SharedState, notify: RepaintNotifier) {
    let Some(path) = path else {
        return;
    };
    tokio::task::spawn_blocking(move || {
        let loaded =
            HistoryReader::open(&path).and_then(|reader| reader.recent_actions(JOURNAL_PREFILL));
        match loaded {
            Ok(records) => {
                if let Ok(mut state) = state.write() {
                    state.actions = records;
                }
                notify();
            }
            Err(error) => tracing::warn!(%error, "action journal was not loaded"),
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn file_content_is_journaled_as_size_only() {
        assert_eq!(
            journaled_argument("files", "write", "secret=1".to_owned()),
            "8 bytes"
        );
        assert_eq!(
            journaled_argument("files", "chmod", "644".to_owned()),
            "644"
        );
        assert_eq!(
            journaled_argument("security", "ban", "x".repeat(200)),
            "200 bytes"
        );
    }
}
