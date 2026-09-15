use std::sync::{Arc, Mutex};

use asiba_core::{Credentials, ServerId, ServerSpec, ServerState};
use asiba_transport::HostKeyPolicy;

use super::{Engine, WorkerEntry};
use crate::command::EngineEvent;
use crate::geo;
use crate::history;
use crate::worker::{self, DocWriter, TransportSlot, WorkerContext};

impl Engine {
    pub(super) async fn save_and_start(&mut self, spec: ServerSpec, credentials: Credentials) {
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

    pub(super) async fn remove(&mut self, id: ServerId) {
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

    pub(super) fn restart(&mut self, id: &ServerId, policy: HostKeyPolicy) {
        let Some(entry) = self.workers.remove(id) else {
            return;
        };
        entry.task.abort();
        self.start_worker(entry.spec, entry.credentials, policy);
    }

    pub(super) fn start_worker(
        &mut self,
        spec: ServerSpec,
        credentials: Credentials,
        policy: HostKeyPolicy,
    ) {
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
            docs: DocWriter::new(
                self.persistence.doc_path(&spec.id),
                self.persistence.llm_doc_path(&spec.id),
            ),
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
}
