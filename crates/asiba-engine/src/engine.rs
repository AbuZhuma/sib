use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use asiba_core::{Credentials, ModuleRegistry, ServerId, ServerSpec, ServerState, SharedState};
use asiba_transport::HostKeyPolicy;
use tokio::sync::mpsc;
use tokio::task::JoinHandle;

use crate::command::{Command, EngineEvent, TestRequest};
use crate::persistence::Persistence;
use crate::test_connection;
use crate::worker::{self, WorkerContext};

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
}

struct Engine {
    registry: ModuleRegistry,
    state: SharedState,
    persistence: Persistence,
    notify: RepaintNotifier,
    events: mpsc::UnboundedSender<EngineEvent>,
    workers: HashMap<ServerId, WorkerEntry>,
}

pub fn spawn(
    runtime: &tokio::runtime::Handle,
    registry: ModuleRegistry,
    state: SharedState,
    persistence: Persistence,
    notify: RepaintNotifier,
) -> EngineHandle {
    let (commands, mut receiver) = mpsc::unbounded_channel();
    let (events, event_receiver) = mpsc::unbounded_channel();
    let mut engine = Engine {
        registry,
        state,
        persistence,
        notify,
        events,
        workers: HashMap::new(),
    };
    runtime.spawn(async move {
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
        let ctx = WorkerContext {
            spec: spec.clone(),
            credentials: credentials.clone(),
            policy,
            registry: self.registry.clone(),
            state: Arc::clone(&self.state),
            notify: Arc::clone(&self.notify),
        };
        let task = tokio::spawn(worker::run(ctx));
        self.workers.insert(
            spec.id.clone(),
            WorkerEntry {
                task,
                spec,
                credentials,
            },
        );
        (self.notify)();
    }

    fn test(&self, request: TestRequest) {
        let registry = self.registry.clone();
        let events = self.events.clone();
        let notify = Arc::clone(&self.notify);
        tokio::spawn(async move {
            let report = test_connection::run(request, registry).await;
            let _ = events.send(EngineEvent::TestFinished(report));
            notify();
        });
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
