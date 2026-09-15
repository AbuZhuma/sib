use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::{Duration, Instant};

use asiba_config::LlmConfig;
use asiba_core::{AuditReport, AuditStatus, ModuleRegistry, SharedState};
use asiba_llm::{Backend, LlamaConfig, LlamaServer};
use chrono::Utc;
use tokio::sync::{mpsc, watch};
use tokio::time::{MissedTickBehavior, interval};

use super::context;
use super::job::AuditJob;
use super::store;
use crate::command::EngineEvent;
use crate::engine::RepaintNotifier;

const IDLE_CHECK: Duration = Duration::from_secs(30);
const MIN_THREADS: usize = 2;
const RESERVED_THREADS: usize = 2;

pub enum WorkerMessage {
    Job(Box<AuditJob>),
    Unload,
}

pub struct AuditWorker {
    pub state: SharedState,
    pub registry: ModuleRegistry,
    pub config: watch::Receiver<LlmConfig>,
    pub audits_dir: PathBuf,
    pub events: mpsc::UnboundedSender<EngineEvent>,
    pub notify: RepaintNotifier,
}

struct Loaded {
    backend: Arc<LlamaServer>,
    last_used: Instant,
}

struct Runner {
    worker: AuditWorker,
    loaded: Option<Loaded>,
    cooldowns: HashMap<String, Instant>,
}

pub fn spawn_detached() -> mpsc::UnboundedSender<WorkerMessage> {
    mpsc::unbounded_channel().0
}

pub fn spawn(worker: AuditWorker) -> mpsc::UnboundedSender<WorkerMessage> {
    let (sender, receiver) = mpsc::unbounded_channel();
    tokio::spawn(run(
        Runner {
            worker,
            loaded: None,
            cooldowns: HashMap::new(),
        },
        receiver,
    ));
    sender
}

async fn run(mut runner: Runner, mut receiver: mpsc::UnboundedReceiver<WorkerMessage>) {
    let mut ticker = interval(IDLE_CHECK);
    ticker.set_missed_tick_behavior(MissedTickBehavior::Delay);
    loop {
        tokio::select! {
            message = receiver.recv() => match message {
                None => break,
                Some(WorkerMessage::Unload) => runner.unload(),
                Some(WorkerMessage::Job(job)) => runner.handle(*job).await,
            },
            _ = ticker.tick() => runner.unload_if_idle(),
        }
    }
}

impl Runner {
    fn unload(&mut self) {
        if self.loaded.take().is_some() {
            tracing::info!("модель выгружена");
            (self.worker.notify)();
        }
    }

    fn unload_if_idle(&mut self) {
        let idle = Duration::from_secs(self.worker.config.borrow().idle_unload_secs);
        if self
            .loaded
            .as_ref()
            .is_some_and(|l| l.last_used.elapsed() > idle)
        {
            self.unload();
        }
    }

    fn should_skip_auto(&mut self, job: &AuditJob, config: &LlmConfig) -> bool {
        if !job.is_auto {
            return false;
        }
        let severity_ok = job
            .incident
            .as_ref()
            .is_some_and(|i| i.severity >= config.auto_audit_min_severity);
        if !config.auto_audit || !severity_ok {
            return true;
        }
        let cooldown = Duration::from_secs(config.cooldown_secs);
        let key = job.cooldown_key();
        if self
            .cooldowns
            .get(&key)
            .is_some_and(|at| at.elapsed() < cooldown)
        {
            return true;
        }
        self.cooldowns.insert(key, Instant::now());
        false
    }

    async fn handle(&mut self, job: AuditJob) {
        let config = self.worker.config.borrow().clone();
        if !config.is_ready() {
            if !job.is_auto {
                self.finish(self.open(&job, String::new()), Err(NOT_READY.to_owned()));
            }
            return;
        }
        if self.should_skip_auto(&job, &config) {
            return;
        }
        let model = config
            .model_path
            .as_ref()
            .map(|p| asiba_llm::model_name(p))
            .unwrap_or_default();
        let report = self.open(&job, model);
        let Some(prepared) =
            context::prepare(&job, &self.worker.state, &self.worker.registry, &config).await
        else {
            self.finish(report, Err(NO_DATA.to_owned()));
            return;
        };
        let backend = match self.ensure_loaded(&config).await {
            Ok(backend) => backend,
            Err(error) => {
                self.finish(report, Err(error));
                return;
            }
        };
        let completion = prepared.completion;
        let result = tokio::task::spawn_blocking(move || backend.complete(&completion))
            .await
            .map_err(|e| e.to_string())
            .and_then(|r| r.map_err(|e| e.to_string()));
        if let Some(loaded) = &mut self.loaded {
            loaded.last_used = Instant::now();
        }
        let mut report = report;
        report.context_tokens = prepared.context_tokens;
        self.finish(report, result);
    }

    async fn ensure_loaded(&mut self, config: &LlmConfig) -> Result<Arc<LlamaServer>, String> {
        if let Some(loaded) = &self.loaded {
            return Ok(Arc::clone(&loaded.backend));
        }
        let llama = llama_config(config)?;
        tracing::info!(model = %llama.model.display(), "запуск llama-server");
        let started = tokio::task::spawn_blocking(move || LlamaServer::start(&llama))
            .await
            .map_err(|e| e.to_string())?
            .map_err(|e| e.to_string())?;
        tracing::info!(port = started.port(), "llama-server готов");
        let backend = Arc::new(started);
        self.loaded = Some(Loaded {
            backend: Arc::clone(&backend),
            last_used: Instant::now(),
        });
        (self.worker.notify)();
        Ok(backend)
    }

    fn open(&self, job: &AuditJob, model: String) -> AuditReport {
        let mut report = AuditReport {
            id: 0,
            server: job.server.clone(),
            scope: job.scope.clone(),
            started_at: Utc::now(),
            finished_at: None,
            model,
            context_tokens: 0,
            text: String::new(),
            status: AuditStatus::Running,
        };
        if let Ok(mut state) = self.worker.state.write() {
            report.id = state.audits.iter().map(|a| a.id).max().unwrap_or(0) + 1;
            state.audits.push(report.clone());
        }
        (self.worker.notify)();
        report
    }

    fn finish(&self, mut report: AuditReport, result: Result<String, String>) {
        report.finished_at = Some(Utc::now());
        match result {
            Ok(text) => {
                report.text = text;
                report.status = AuditStatus::Done;
                store::write_report(&self.worker.audits_dir, &report);
            }
            Err(error) => {
                tracing::warn!(server = %report.server, "аудит не выполнен: {error}");
                report.status = AuditStatus::Failed(error);
            }
        }
        if let Ok(mut state) = self.worker.state.write()
            && let Some(stored) = state.audit_mut(report.id)
        {
            *stored = report.clone();
        }
        let _ = self.worker.events.send(EngineEvent::AuditFinished(report));
        (self.worker.notify)();
    }
}

const NOT_READY: &str = "локальная модель выключена или файл модели не найден";
const NO_DATA: &str = "нет данных сервера для аудита";

fn llama_config(config: &LlmConfig) -> Result<LlamaConfig, String> {
    let model = config
        .model_path
        .clone()
        .ok_or_else(|| NOT_READY.to_owned())?;
    let threads = if config.threads == 0 {
        std::thread::available_parallelism()
            .map(|n| n.get().saturating_sub(RESERVED_THREADS))
            .unwrap_or(MIN_THREADS)
            .max(MIN_THREADS)
    } else {
        config.threads
    };
    Ok(LlamaConfig {
        binary: PathBuf::from(&config.server_binary),
        model,
        threads,
        context_tokens: config.context_tokens,
    })
}
