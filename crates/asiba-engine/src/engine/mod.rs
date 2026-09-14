mod on_demand;

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use asiba_core::{
    Credentials, Intervals, ModuleRegistry, ServerId, ServerSpec, ServerState, SharedState,
};
use asiba_storage::StorageWriter;
use asiba_transport::HostKeyPolicy;
use tokio::sync::{mpsc, watch};
use tokio::task::JoinHandle;

use crate::actions;
use crate::alerts::{self, AlertLoop, AlertSettings};
use crate::command::{Command, EngineEvent};
use crate::geo;
use crate::history;
use crate::peers;
use crate::persistence::Persistence;
use crate::worker::{self, DocWriter, TransportSlot, WorkerContext};

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
}

pub fn spawn(
    runtime: &tokio::runtime::Handle,
    deps: EngineDeps,
    notify: RepaintNotifier,
) -> EngineHandle {
    let (commands, mut receiver) = mpsc::unbounded_channel();
    let (events, event_receiver) = mpsc::unbounded_channel();
    let (alert_settings, alert_receiver) = watch::channel(deps.alert_settings);
    let alert_loop = AlertLoop {
        state: Arc::clone(&deps.state),
        settings: alert_receiver,
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
    };
    runtime.spawn(async move {
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
        }
    }

    async fn save_and_start(&mut self, spec: ServerSpec, credentials: Credentials) {
        let persistence = self.persistence.clone();
        let (spec_copy, creds_copy) = (spec.clone(), credentials.clone());
        let saved =
            tokio::task::spawn_blocking(move || persistence.save(&spec_copy, &creds_copy)).await;
        match saved {
            Ok(Ok(())) => self.emit(EngineEvent::ServerSaved(spec.id.clone())),
            Ok(Err(error)) => self.warn(format!("не удалось сохранить сервер: {error}")),
            Err(error) => self.warn(format!("сбой сохранения: {error}")),
        }
        self.start_worker(spec, credentials, HostKeyPolicy::KnownHostsOnly);
    }

    async fn remove(&mut self, id: ServerId) {
        if let Some(entry) = self.workers.remove(&id) {
            entry.task.abort();
        }
        if let Ok(mut state) = self.state.write() {
            state.servers.remove(&id);
        }
        let persistence = self.persistence.clone();
        let id_copy = id.clone();
        let removed = tokio::task::spawn_blocking(move || persistence.remove(&id_copy)).await;
        if let Ok(Err(error)) = removed {
            self.warn(format!("не удалось удалить файлы сервера: {error}"));
        }
        self.emit(EngineEvent::ServerRemoved(id));
    }

    fn restart(&mut self, id: &ServerId, policy: HostKeyPolicy) {
        let Some(entry) = self.workers.remove(id) else {
            return;
        };
        entry.task.abort();
        self.start_worker(entry.spec, entry.credentials, policy);
    }

    fn start_worker(&mut self, spec: ServerSpec, credentials: Credentials, policy: HostKeyPolicy) {
        if let Some(previous) = self.workers.remove(&spec.id) {
            previous.task.abort();
        }
        if let Ok(mut state) = self.state.write() {
            state
                .servers
                .insert(spec.id.clone(), ServerState::new(spec.clone()));
        }
        let transport: TransportSlot = Arc::new(Mutex::new(None));
        let ctx = WorkerContext {
            spec: spec.clone(),
            intervals: self.intervals,
            credentials: credentials.clone(),
            policy,
            registry: self.registry.clone(),
            state: Arc::clone(&self.state),
            notify: Arc::clone(&self.notify),
            storage: self.storage.clone(),
            transport: Arc::clone(&transport),
            docs: DocWriter::new(self.persistence.doc_path(&spec.id)),
        };
        let task = tokio::spawn(worker::run(ctx));
        let prefill = history::Prefill {
            path: self.history_path.clone(),
            state: Arc::clone(&self.state),
            server: spec.id.clone(),
            notify: Arc::clone(&self.notify),
        };
        history::prefill(prefill);
        geo::resolve_server(self.geo_request(), spec.clone());
        let entry = WorkerEntry {
            task,
            spec: spec.clone(),
            credentials,
            transport,
        };
        self.workers.insert(spec.id, entry);
        (self.notify)();
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
