mod audits;
mod on_demand;
mod spawn;
mod workers;

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

use asiba_config::AiConfig;
use asiba_core::{
    Credentials, CustomCheck, IgnoredIncident, Intervals, ModuleId, ModuleRegistry, ServerId,
    ServerSpec, SharedState,
};
use asiba_storage::StorageWriter;
use asiba_transport::HostKeyPolicy;
use tokio::sync::{broadcast, mpsc, watch};
use tokio::task::JoinHandle;

use crate::actions;
use crate::ai::{AuditJob, Cancellations};
use crate::alerts::{self, AlertSettings};
use crate::command::{Command, EngineEvent};
use crate::geo;
use crate::persistence::Persistence;
use crate::worker::TransportSlot;

pub use spawn::spawn;

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
    pub geolocation: bool,
    pub checks: asiba_modules::checks::Defined,
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
    geolocation: Arc<AtomicBool>,
    checks: asiba_modules::checks::Defined,
    intervals: Intervals,
    notify: RepaintNotifier,
    events: mpsc::UnboundedSender<EngineEvent>,
    workers: HashMap<ServerId, WorkerEntry>,
    alert_settings: watch::Sender<AlertSettings>,
    ai_config: watch::Sender<AiConfig>,
    audits: mpsc::UnboundedSender<Box<AuditJob>>,
    cancellations: Cancellations,
}

impl Engine {
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

    fn set_checks(&mut self, checks: Vec<CustomCheck>) {
        self.checks.set(checks);
        let ids: Vec<ServerId> = self.workers.keys().cloned().collect();
        for id in ids {
            self.restart(&id, HostKeyPolicy::KnownHostsOnly);
        }
    }

    fn geo_request(&self) -> geo::GeoRequest {
        geo::GeoRequest {
            cache: self.geo_cache.clone(),
            enabled: Arc::clone(&self.geolocation),
            state: Arc::clone(&self.state),
            notify: Arc::clone(&self.notify),
        }
    }

    fn set_geolocation(&self, enabled: bool) {
        let was_enabled = self.geolocation.swap(enabled, Ordering::Relaxed);
        if enabled && !was_enabled {
            geo::resolve_self(self.geo_request());
            for entry in self.workers.values() {
                geo::resolve_server(self.geo_request(), entry.spec.clone());
            }
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
            Command::UpdateServer {
                previous,
                spec,
                credentials,
            } => self.update(previous, spec, credentials).await,
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
            other => self.handle_settings(other),
        }
    }

    async fn update(&mut self, previous: ServerId, spec: ServerSpec, credentials: Credentials) {
        let merged = match self.workers.get(&previous) {
            Some(entry) => credentials.fill_missing_from(&entry.credentials),
            None => credentials,
        };
        if previous != spec.id {
            self.rename(&previous, &spec.id).await;
        }
        self.save_and_start(spec, merged).await;
    }

    fn handle_settings(&mut self, command: Command) {
        match command {
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
            Command::SetGeolocation(enabled) => self.set_geolocation(enabled),
            Command::SetChecks(checks) => self.set_checks(checks),
            Command::SetRetention(retention) => {
                if let Some(storage) = &self.storage
                    && storage.set_retention(retention).is_err()
                {
                    self.warn("хранилище остановлено, сроки хранения не применены".to_owned());
                }
            }
            Command::Audit {
                target,
                scope,
                is_auto,
            } => self.audit(target, scope, is_auto),
            Command::SetAiConfig(config) => self.set_ai_config(config),
            Command::CancelAudit(id) => self.cancel_audit(id),
            Command::SetIgnoredIncidents(ignored) => self.set_ignored(ignored),
            _ => {}
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
