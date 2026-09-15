use std::sync::Arc;

use asiba_core::{ActionRequest, ModuleId, QueryRequest, ServerId, Transport};

use super::Engine;
use crate::actions::{self, Perform};
use crate::command::{EngineEvent, TestRequest};
use crate::test_connection;

impl Engine {
    pub(super) fn test(&self, request: TestRequest) {
        let registry = self.registry.clone();
        let events = self.events.clone();
        let notify = Arc::clone(&self.notify);
        tokio::spawn(async move {
            let report = test_connection::run(request, registry).await;
            let _ = events.send(EngineEvent::TestFinished(report));
            notify();
        });
    }

    pub(super) fn query(
        &self,
        token: u64,
        server: &ServerId,
        module: ModuleId,
        request: QueryRequest,
    ) {
        let transport = self.transport_of(server);
        let module = self.registry.get(module).cloned();
        let events = self.events.clone();
        let notify = Arc::clone(&self.notify);
        tokio::spawn(async move {
            let result = match (transport, module) {
                (Some(transport), Some(module)) => module
                    .query(transport.as_ref(), &request)
                    .await
                    .map_err(|e| e.to_string()),
                (None, _) => Err("сервер не подключён".to_owned()),
                (_, None) => Err("модуль не найден".to_owned()),
            };
            let _ = events.send(EngineEvent::QueryFinished { token, result });
            notify();
        });
    }

    pub(super) fn backfill(&self, server: &ServerId, module: ModuleId) {
        if let Some(entry) = self.workers.get(server) {
            let _ = entry.backfill.send(module);
        }
    }

    pub(super) fn perform(&self, server: &ServerId, module: ModuleId, request: ActionRequest) {
        let transport = self.transport_of(server);
        actions::perform(Perform {
            server: server.clone(),
            module: self.registry.get(module).cloned(),
            transport,
            request,
            state: Arc::clone(&self.state),
            storage: self.storage.clone(),
            events: self.events.clone(),
            notify: Arc::clone(&self.notify),
        });
    }

    pub(super) fn transport_of(&self, server: &ServerId) -> Option<Arc<dyn Transport>> {
        self.workers
            .get(server)
            .and_then(|entry| entry.transport.lock().ok().and_then(|slot| slot.clone()))
    }
}
