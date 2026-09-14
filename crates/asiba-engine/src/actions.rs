use std::path::PathBuf;
use std::sync::Arc;

use asiba_core::{ActionRecord, ActionRequest, Module, ServerId, SharedState, Transport};
use asiba_storage::{HistoryReader, StorageWriter};
use chrono::Utc;
use tokio::sync::mpsc;

use crate::command::EngineEvent;
use crate::engine::RepaintNotifier;

const JOURNAL_PREFILL: usize = 200;

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
        let result = match (&job.transport, &job.module) {
            (Some(transport), Some(module)) => module
                .perform(transport.as_ref(), &job.request)
                .await
                .map(|outcome| outcome.message)
                .map_err(|e| e.to_string()),
            (None, _) => Err("сервер не подключён".to_owned()),
            (_, None) => Err("модуль не найден".to_owned()),
        };
        let record = ActionRecord {
            at: Utc::now(),
            server: job.server,
            module: module_name,
            kind: job.request.kind,
            target: job.request.target,
            argument: job.request.argument,
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

fn log_record(record: &ActionRecord) {
    if record.is_success {
        tracing::info!(server = %record.server, kind = %record.kind, target = %record.target, "действие выполнено");
    } else {
        tracing::warn!(server = %record.server, kind = %record.kind, target = %record.target, error = %record.message, "действие не выполнено");
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
            Err(error) => tracing::warn!(%error, "журнал действий не загружен"),
        }
    });
}
