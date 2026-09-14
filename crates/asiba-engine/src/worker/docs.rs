use std::path::PathBuf;
use std::sync::Mutex;
use std::time::{Duration, Instant};

use asiba_core::{ModuleId, SharedState};

use super::WorkerContext;

const MIN_INTERVAL: Duration = Duration::from_secs(60);

#[derive(Default)]
pub struct DocState {
    last_written: Option<Instant>,
    last_body: String,
}

pub struct DocWriter {
    path: PathBuf,
    state: Mutex<DocState>,
}

impl DocWriter {
    pub fn new(path: PathBuf) -> Self {
        Self {
            path,
            state: Mutex::new(DocState::default()),
        }
    }

    pub fn maybe_write(&self, ctx: &WorkerContext, module: ModuleId) {
        if !asiba_docgen::is_trigger(module) {
            return;
        }
        let Ok(mut state) = self.state.lock() else {
            return;
        };
        if state
            .last_written
            .is_some_and(|at| at.elapsed() < MIN_INTERVAL)
        {
            return;
        }
        let Some(rendered) = render(&ctx.state, ctx) else {
            return;
        };
        let body = asiba_docgen::strip_timestamp(&rendered).to_owned();
        if body == state.last_body {
            return;
        }
        let existing = std::fs::read_to_string(&self.path).ok();
        let content = asiba_docgen::merge_notes(existing.as_deref(), &rendered);
        if let Some(parent) = self.path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        match std::fs::write(&self.path, content) {
            Ok(()) => {
                state.last_written = Some(Instant::now());
                state.last_body = body;
            }
            Err(error) => {
                tracing::warn!(path = %self.path.display(), %error, "файл сервера не записан")
            }
        }
    }
}

fn render(shared: &SharedState, ctx: &WorkerContext) -> Option<String> {
    let state = shared.read().ok()?;
    let server = state.servers.get(&ctx.spec.id)?;
    if !asiba_docgen::has_required_data(server) {
        return None;
    }
    Some(asiba_docgen::render(server))
}
