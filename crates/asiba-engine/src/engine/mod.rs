mod on_demand;
mod workers;

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use asiba_config::AiConfig;
use asiba_core::{
    AuditReport, AuditScope, AuditStatus, AuditTarget, Credentials, IgnoredIncident, Intervals,
    ModuleId, ModuleRegistry, ServerId, ServerSpec, SharedState,
};
use asiba_storage::StorageWriter;
use asiba_transport::HostKeyPolicy;
use chrono::Utc;
use tokio::sync::{broadcast, mpsc, watch};
use tokio::task::JoinHandle;

use crate::actions;
use crate::ai::{self, AuditJob, AuditWorker, Cancellations, StartupSummary};
use crate::alerts::{self, AlertLoop, AlertSettings};
use crate::command::{Command, EngineEvent};
use crate::geo;
use crate::peers;
use crate::persistence::Persistence;
use crate::worker::TransportSlot;

pub type RepaintNotifier = Arc<dyn Fn() + Send + Sync>;

pub struct EngineHandle {
    commands: mpsc::UnboundedSender<Command>,
    events: Mutex<mpsc::UnboundedReceiver<EngineEvent>>,
}

impl EngineHandle {
    pub fn send(&self, command: Command) {
        if self.commands.send(command).is_err() {
            tracing::error!("движок остановлен, команда потеряна");
        }
    }

    pub fn poll_events(&self) -> Vec<EngineEvent> {
        let mut drained = Vec::new();
        if let Ok(mut receiver) = self.events.lock() {
            while let Ok(event) = receiver.try_recv() {
                drained.push(event);
            }
        }
        drained
    }
}

struct WorkerEntry {
    task: JoinHandle<()>,
    spec: ServerSpec,
    credentials: Credentials,
    transport: TransportSlot,
    backfill: broadcast::Sender<ModuleId>,
}

pub struct EngineDeps {
    pub registry: ModuleRegistry,
    pub state: SharedState,
    pub persistence: Persistence,
    pub storage: Option<StorageWriter>,
    pub history_path: Option<PathBuf>,
    pub geo_cache: Option<PathBuf>,
    pub alert_settings: AlertSettings,
    pub intervals: Intervals,
    pub ai: AiConfig,
    pub audits_dir: PathBuf,
    pub ignored: Vec<IgnoredIncident>,
}

struct Engine {
    registry: ModuleRegistry,
    state: SharedState,
    persistence: Persistence,
    storage: Option<StorageWriter>,
    history_path: Option<PathBuf>,
    geo_cache: Option<PathBuf>,
    intervals: Intervals,
    notify: RepaintNotifier,
    events: mpsc::UnboundedSender<EngineEvent>,
    workers: HashMap<ServerId, WorkerEntry>,
    alert_settings: watch::Sender<AlertSettings>,
    ai_config: watch::Sender<AiConfig>,
    audits: mpsc::UnboundedSender<Box<AuditJob>>,
    cancellations: Cancellations,
}

pub fn spawn(
    runtime: &tokio::runtime::Handle,
    deps: EngineDeps,
    notify: RepaintNotifier,
) -> EngineHandle {
    let (commands, mut receiver) = mpsc::unbounded_channel();
    let (events, event_receiver) = mpsc::unbounded_channel();
    let (alert_settings, alert_receiver) = watch::channel(deps.alert_settings);
    let (ai_config, ai_receiver) = watch::channel(deps.ai);
    let alert_loop = AlertLoop {
        state: Arc::clone(&deps.state),
        settings: alert_receiver,
        ai: ai_receiver.clone(),
        notify: Arc::clone(&notify),
        events: events.clone(),
        commands: commands.clone(),
    };
    let startup_summary = StartupSummary {
        state: Arc::clone(&deps.state),
        config: ai_receiver.clone(),
        commands: commands.clone(),
    };
    let deps_state = Arc::clone(&deps.state);
    let cancellations = Cancellations::default();
    let audit_worker = AuditWorker {
        cancellations: cancellations.clone(),
        state: Arc::clone(&deps.state),
        registry: deps.registry.clone(),
        config: ai_receiver,
        audits_dir: deps.audits_dir,
        events: events.clone(),
        notify: Arc::clone(&notify),
    };
    let mut engine = Engine {
        registry: deps.registry,
        state: deps.state,
        persistence: deps.persistence,
        storage: deps.storage,
        history_path: deps.history_path,
        geo_cache: deps.geo_cache,
        intervals: deps.intervals.clamped(),
        notify,
        events,
        workers: HashMap::new(),
        alert_settings,
        ai_config,
        audits: ai::spawn_detached(),
        cancellations,
    };
    if let Ok(mut state) = deps_state.write() {
        state.ignored_incidents = deps.ignored;
    }
    runtime.spawn(async move {
        engine.audits = ai::spawn(audit_worker);
        ai::startup::spawn(startup_summary);
        alerts::spawn(alert_loop);
        geo::resolve_self(engine.geo_request());
        peers::spawn(peers::PeerLookup {
            cache: engine.geo_cache.clone(),
            state: Arc::clone(&engine.state),
            notify: Arc::clone(&engine.notify),
        });
        engine.load_action_journal();
        engine.load_saved().await;
        while let Some(command) = receiver.recv().await {
            engine.handle(command).await;
        }
    });
    EngineHandle {
        commands,
        events: Mutex::new(event_receiver),
    }
}

impl Engine {
    fn audit(&self, target: AuditTarget, scope: AuditScope, is_auto: bool) {
        let incident = match &scope {
            AuditScope::Incident { incident_id, .. } => self
                .state
                .read()
                .ok()
                .and_then(|s| s.incidents.iter().find(|i| i.id == *incident_id).cloned()),
            AuditScope::Full | AuditScope::Section { .. } => None,
        };
        let job = AuditJob {
            report_id: self.enqueue_report(&target, &scope),
            transport: target.server().and_then(|id| self.transport_of(id)),
            target,
            scope,
            incident,
            is_auto,
        };
        let _ = self.audits.send(Box::new(job));
    }

    fn set_ignored(&self, ignored: Vec<IgnoredIncident>) {
        if let Ok(mut state) = self.state.write() {
            state.ignored_incidents = ignored;
            let now = Utc::now();
            let reconciled = asiba_incidents::reconcile(&mut state, now);
            tracing::info!(
                resolved = reconciled.resolved.len(),
                "список игнорируемых инцидентов обновлён"
            );
        }
        (self.notify)();
    }

    fn cancel_audit(&self, id: u64) {
        if let Ok(mut state) = self.state.write()
            && state.audits.iter().any(|a| a.id == id && a.is_running())
        {
            state.audits.retain(|a| a.id != id);
            self.cancellations.cancel(id);
        }
        (self.notify)();
    }

    fn enqueue_report(&self, target: &AuditTarget, scope: &AuditScope) -> u64 {
        let Ok(mut state) = self.state.write() else {
            return 0;
        };
        let id = state.audits.iter().map(|a| a.id).max().unwrap_or(0) + 1;
        state.audits.push(AuditReport {
            id,
            target: target.clone(),
            scope: scope.clone(),
            started_at: Utc::now(),
            finished_at: None,
            model: self.ai_config.borrow().model.clone(),
            context_tokens: 0,
            text: String::new(),
            status: AuditStatus::Queued,
        });
        drop(state);
        (self.notify)();
        id
    }

    fn set_ai_config(&mut self, config: AiConfig) {
        let became_ready = config.is_ready() && !self.ai_config.borrow().is_ready();
        let _ = self.ai_config.send(config);
        let has_summary = self
            .state
            .read()
            .ok()
            .is_some_and(|s| s.audits.iter().any(|a| a.target == AuditTarget::Fleet));
        if became_ready && !has_summary {
            self.audit(AuditTarget::Fleet, AuditScope::Full, true);
        }
    }

    fn set_intervals(&mut self, intervals: Intervals) {
        let clamped = intervals.clamped();
        if clamped == self.intervals {
            return;
        }
        self.intervals = clamped;
        let ids: Vec<ServerId> = self.workers.keys().cloned().collect();
        for id in ids {
            self.restart(&id, HostKeyPolicy::KnownHostsOnly);
        }
    }

    fn geo_request(&self) -> geo::GeoRequest {
        geo::GeoRequest {
            cache: self.geo_cache.clone(),
            state: Arc::clone(&self.state),
            notify: Arc::clone(&self.notify),
        }
    }

    fn load_action_journal(&self) {
        actions::prefill_journal(
            self.history_path.clone(),
            Arc::clone(&self.state),
            Arc::clone(&self.notify),
        );
    }

    async fn load_saved(&mut self) {
        let persistence = self.persistence.clone();
        let loaded = tokio::task::spawn_blocking(move || persistence.load_all()).await;
        match loaded {
            Ok(Ok(servers)) => {
                for (spec, credentials) in servers {
                    self.start_worker(spec, credentials, HostKeyPolicy::KnownHostsOnly);
                }
            }
            Ok(Err(error)) => self.warn(format!("не удалось загрузить серверы: {error}")),
            Err(error) => self.warn(format!("сбой загрузки: {error}")),
        }
    }

    async fn handle(&mut self, command: Command) {
        match command {
            Command::AddServer { spec, credentials } => {
                self.save_and_start(spec, credentials).await;
            }
            Command::UpdateServer { spec, credentials } => {
                let merged = match self.workers.get(&spec.id) {
                    Some(entry) => credentials.fill_missing_from(&entry.credentials),
                    None => credentials,
                };
                self.save_and_start(spec, merged).await;
            }
            Command::RemoveServer(id) => self.remove(id).await,
            Command::Reconnect(id) => self.restart(&id, HostKeyPolicy::KnownHostsOnly),
            Command::TrustHostKey {
                server,
                fingerprint,
            } => {
                self.restart(&server, HostKeyPolicy::TrustFingerprint(fingerprint));
            }
            Command::TestConnection(request) => self.test(request),
            Command::Query {
                token,
                server,
                module,
                request,
            } => {
                self.query(token, &server, module, request);
            }
            Command::Backfill { server, module } => self.backfill(&server, module),
            Command::Perform {
                server,
                module,
                request,
            } => self.perform(&server, module, request),
            Command::AcknowledgeAlert(id) => {
                alerts::acknowledge(&self.state, id);
                (self.notify)();
            }
            Command::MuteAlert { id, until } => {
                alerts::mute(&self.state, id, until);
                (self.notify)();
            }
            Command::SetAlertSettings(settings) => {
                let _ = self.alert_settings.send(settings);
            }
            Command::SetIntervals(intervals) => self.set_intervals(intervals),
            Command::Audit {
                target,
                scope,
                is_auto,
            } => self.audit(target, scope, is_auto),
            Command::SetAiConfig(config) => self.set_ai_config(config),
            Command::CancelAudit(id) => self.cancel_audit(id),
            Command::SetIgnoredIncidents(ignored) => self.set_ignored(ignored),
        }
    }

    fn emit(&self, event: EngineEvent) {
        let _ = self.events.send(event);
        (self.notify)();
    }

    fn warn(&self, message: String) {
        tracing::warn!("{message}");
        self.emit(EngineEvent::Warning(message));
    }
}
