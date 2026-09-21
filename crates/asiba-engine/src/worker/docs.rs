use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::time::{Duration, Instant};

use asiba_docgen::DocContext;

use super::WorkerContext;

const MIN_INTERVAL: Duration = Duration::from_secs(10);

#[derive(Default)]
struct DocState {
    last_written: Option<Instant>,
    last_human_body: String,
    last_llm_body: String,
}

pub struct DocWriter {
    human_path: PathBuf,
    llm_path: PathBuf,
    state: Mutex<DocState>,
}

struct Rendered {
    human: String,
    llm: String,
}

impl DocWriter {
    pub fn new(human_path: PathBuf, llm_path: PathBuf) -> Self {
        Self {
            human_path,
            llm_path,
            state: Mutex::new(DocState::default()),
        }
    }

    pub fn maybe_write(&self, ctx: &WorkerContext) {
        let Ok(mut state) = self.state.try_lock() else {
            return;
        };
        if state
            .last_written
            .is_some_and(|at| at.elapsed() < MIN_INTERVAL)
        {
            return;
        }
        let Some(rendered) = render(ctx) else {
            return;
        };
        let human_body = asiba_docgen::strip_timestamp(&rendered.human).to_owned();
        let llm_body = asiba_docgen::strip_timestamp(&rendered.llm).to_owned();
        if human_body == state.last_human_body && llm_body == state.last_llm_body {
            return;
        }
        let existing = std::fs::read_to_string(&self.human_path).ok();
        let human = asiba_docgen::merge_notes(existing.as_deref(), &rendered.human);
        if write_file(&self.human_path, &human) && write_file(&self.llm_path, &rendered.llm) {
            state.last_written = Some(Instant::now());
            state.last_human_body = human_body;
            state.last_llm_body = llm_body;
        }
    }
}

fn write_file(path: &Path, content: &str) -> bool {
    if let Some(parent) = path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    match std::fs::write(path, content) {
        Ok(()) => true,
        Err(error) => {
            tracing::warn!(path = %path.display(), %error, "файл сервера не записан");
            false
        }
    }
}

fn render(ctx: &WorkerContext) -> Option<Rendered> {
    let state = ctx.state.read().ok()?;
    let server = state.servers.get(&ctx.spec.id)?;
    if !asiba_docgen::has_required_data(server) {
        return None;
    }
    let doc = DocContext::new(server, &state);
    Some(Rendered {
        human: asiba_docgen::render_human(&doc),
        llm: asiba_docgen::render_llm(&doc),
    })
}
