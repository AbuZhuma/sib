use std::path::PathBuf;

use asiba_core::{ServerId, SharedState};
use asiba_storage::HistoryReader;
use chrono::{Duration, Utc};

use crate::engine::RepaintNotifier;

const PREFILL_WINDOW: Duration = Duration::minutes(30);

pub struct Prefill {
    pub path: Option<PathBuf>,
    pub state: SharedState,
    pub server: ServerId,
    pub notify: RepaintNotifier,
}

pub fn prefill(request: Prefill) {
    let Prefill {
        path,
        state,
        server,
        notify,
    } = request;
    let Some(path) = path else {
        return;
    };
    tokio::task::spawn_blocking(move || {
        let since = Utc::now() - PREFILL_WINDOW;
        let loaded = HistoryReader::open(&path)
            .and_then(|reader| reader.load_series(server.as_str(), since));
        match loaded {
            Ok(history) => {
                let points: usize = history.values().map(Vec::len).sum();
                tracing::info!(%server, points, "история метрик загружена");
                if let Ok(mut state) = state.write()
                    && let Some(entry) = state.servers.get_mut(&server)
                {
                    entry.prepend_history(history);
                }
                notify();
            }
            Err(error) => tracing::warn!(%server, %error, "история метрик не загружена"),
        }
    });
}
