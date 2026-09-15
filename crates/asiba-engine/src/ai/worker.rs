use std::collections::HashMap;
use std::path::PathBuf;
use std::time::{Duration, Instant};

use asiba_config::AiConfig;
use asiba_core::{AuditReport, AuditStatus, ModuleRegistry, SharedState};
use chrono::Utc;
use tokio::sync::{mpsc, watch};

use super::backend;
use super::context;
use super::job::AuditJob;
use super::store;
use crate::command::EngineEvent;
use crate::engine::RepaintNotifier;

pub struct AuditWorker {
    pub state: SharedState,
    pub registry: ModuleRegistry,
    pub config: watch::Receiver<AiConfig>,
    pub audits_dir: PathBuf,
    pub events: mpsc::UnboundedSender<EngineEvent>,
    pub notify: RepaintNotifier,
}

struct Runner {
    worker: AuditWorker,
    cooldowns: HashMap<String, Instant>,
}

pub fn spawn_detached() -> mpsc::UnboundedSender<Box<AuditJob>> {
    mpsc::unbounded_channel().0
}

pub fn spawn(worker: AuditWorker) -> mpsc::UnboundedSender<Box<AuditJob>> {
    let (sender, receiver) = mpsc::unbounded_channel();
    tokio::spawn(run(
        Runner {
            worker,
            cooldowns: HashMap::new(),
        },
        receiver,
    ));
    sender
}

async fn run(mut runner: Runner, mut receiver: mpsc::UnboundedReceiver<Box<AuditJob>>) {
    while let Some(job) = receiver.recv().await {
        runner.handle(*job).await;
    }
}

impl Runner {
    fn should_skip_auto(&mut self, job: &AuditJob, config: &AiConfig) -> bool {
        if !job.is_auto {
            return false;
        }
        let severity_ok = job
            .incident
            .as_ref()
            .is_none_or(|i| i.severity >= config.auto_audit_min_severity);
        if job.scope.is_incident() && (!config.auto_audit || !severity_ok) {
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
        let report = self.open(&job, config.model.clone());
        let Some(prepared) =
            context::prepare(&job, &self.worker.state, &self.worker.registry, &config).await
        else {
            self.finish(report, Err(NO_DATA.to_owned()));
            return;
        };
        let backend = match backend::build(&config) {
            Ok(backend) => backend,
            Err(error) => {
                self.finish(report, Err(error.to_string()));
                return;
            }
        };
        let completion = prepared.completion;
        let result = tokio::task::spawn_blocking(move || backend.complete(&completion))
            .await
            .map_err(|e| e.to_string())
            .and_then(|r| r.map_err(|e| e.to_string()));
        let mut report = report;
        report.context_tokens = prepared.context_tokens;
        self.finish(report, result);
    }

    fn open(&self, job: &AuditJob, model: String) -> AuditReport {
        let mut report = AuditReport {
            id: 0,
            target: job.target.clone(),
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
                tracing::warn!(target = %report.target.key(), "аудит не выполнен: {error}");
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

const NOT_READY: &str = "ИИ-анализ выключен: включите его в настройках и укажите ключ API";
const NO_DATA: &str = "нет данных сервера для аудита";
