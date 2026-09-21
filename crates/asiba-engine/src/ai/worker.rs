use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::{Duration, Instant};

use asiba_ai::{AiError, Completion, RETRY_DELAYS, complete_with_retry};
use asiba_config::AiConfig;
use asiba_core::{AuditReport, AuditStatus, ModuleRegistry, SharedState};
use chrono::Utc;
use tokio::sync::{mpsc, watch};

use super::backend;
use super::cancel::Cancellations;
use super::context;
use super::job::AuditJob;
use super::store;
use crate::command::EngineEvent;
use crate::engine::RepaintNotifier;

pub struct AuditWorker {
    pub cancellations: Cancellations,
    pub state: SharedState,
    pub registry: ModuleRegistry,
    pub config: watch::Receiver<AiConfig>,
    pub audits_dir: PathBuf,
    pub events: mpsc::UnboundedSender<EngineEvent>,
    pub notify: RepaintNotifier,
}

const QUOTA_STATUS: u16 = 429;
const QUOTA_PAUSE: Duration = Duration::from_secs(15 * 60);

struct Runner {
    worker: AuditWorker,
    cooldowns: HashMap<String, Instant>,
    quota_paused_until: Option<Instant>,
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
            quota_paused_until: None,
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
        if self
            .quota_paused_until
            .is_some_and(|until| Instant::now() < until)
        {
            return true;
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
        if self.worker.cancellations.take(job.report_id) {
            return;
        }
        let config = self.worker.config.borrow().clone();
        if !config.is_ready() {
            if job.is_auto {
                self.drop_queued(job.report_id);
            } else {
                self.finish(self.open(&job, String::new()), Err(NOT_READY.to_owned()));
            }
            return;
        }
        if self.should_skip_auto(&job, &config) {
            self.drop_queued(job.report_id);
            return;
        }
        let report = self.open(&job, config.model.clone());
        let Some(prepared) =
            context::prepare(&job, &self.worker.state, &self.worker.registry, &config).await
        else {
            self.finish(report, Err(NO_DATA.to_owned()));
            return;
        };
        let mut report = report;
        report.context_tokens = prepared.context_tokens;
        let completion = Arc::new(prepared.completion);
        let report_id = report.id;
        let result = tokio::select! {
            result = complete_with_fallbacks(&config, completion, &mut report) => result,
            _ = self.worker.cancellations.wait_for(report_id) => {
                self.worker.cancellations.take(report_id);
                return;
            }
        };
        if let Err(AiError::Api { status, .. }) = &result
            && *status == QUOTA_STATUS
        {
            tracing::warn!("квота ИИ исчерпана, автозапросы приостановлены на 15 минут");
            self.quota_paused_until = Some(Instant::now() + QUOTA_PAUSE);
        }
        self.finish(report, result.map_err(|e| e.to_string()));
    }

    fn open(&self, job: &AuditJob, model: String) -> AuditReport {
        let mut report = AuditReport {
            id: job.report_id,
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
            match state.audit_mut(job.report_id) {
                Some(stored) => *stored = report.clone(),
                None => {
                    report.id = state.audits.iter().map(|a| a.id).max().unwrap_or(0) + 1;
                    state.push_audit(report.clone());
                }
            }
        }
        (self.worker.notify)();
        report
    }

    fn drop_queued(&self, report_id: u64) {
        if let Ok(mut state) = self.worker.state.write() {
            state.audits.retain(|a| a.id != report_id);
        }
        (self.worker.notify)();
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

async fn complete_with_fallbacks(
    config: &AiConfig,
    completion: Arc<Completion>,
    report: &mut AuditReport,
) -> Result<String, AiError> {
    let mut result = complete(config, &config.model, Arc::clone(&completion)).await;
    for fallback in config.provider.fallback_models() {
        let is_transient = result.as_ref().is_err_and(AiError::is_transient);
        if !is_transient || *fallback == config.model {
            continue;
        }
        tracing::warn!(model = fallback, "модель недоступна, пробуем запасную");
        report.model = (*fallback).to_owned();
        result = complete(config, fallback, Arc::clone(&completion)).await;
    }
    result
}

async fn complete(
    config: &AiConfig,
    model: &str,
    completion: Arc<Completion>,
) -> Result<String, AiError> {
    let backend = backend::build(config, model)?;
    tokio::task::spawn_blocking(move || {
        complete_with_retry(backend.as_ref(), &completion, &RETRY_DELAYS)
    })
    .await
    .map_err(|e| AiError::Request(e.to_string()))?
}

const NOT_READY: &str = "ИИ-анализ выключен: включите его в настройках и укажите ключ API";
const NO_DATA: &str = "нет данных сервера для аудита";
