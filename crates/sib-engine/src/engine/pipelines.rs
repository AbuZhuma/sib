use std::collections::BTreeMap;
use std::sync::Arc;

use chrono::Utc;
use sib_core::{Pipeline, PipelineBinding, ServerId};

use super::Engine;
use crate::pipelines::{self, RunJob};

impl Engine {
    pub(super) fn run_pipeline(
        &self,
        server: &ServerId,
        pipeline: Pipeline,
        values: BTreeMap<String, String>,
    ) {
        let Some(entry) = self.workers.get(server) else {
            self.warn(format!("server {server} is not known"));
            return;
        };
        if let Err(error) = pipeline.validate() {
            self.warn(format!("pipeline '{}': {error}", pipeline.name));
            return;
        }
        pipelines::start(RunJob {
            run_id: next_run_id(),
            spec: entry.spec.clone(),
            pipeline,
            values,
            transport: self.transport_of(server),
            state: Arc::clone(&self.state),
            storage: self.storage.clone(),
            events: self.events.clone(),
            notify: Arc::clone(&self.notify),
            cancellations: self.cancellations.clone(),
        });
    }

    pub(super) async fn set_server_pipelines(
        &mut self,
        id: ServerId,
        bindings: Vec<PipelineBinding>,
    ) {
        let Some(entry) = self.workers.get_mut(&id) else {
            return;
        };
        entry.spec.pipelines = bindings.clone();
        if let Ok(mut state) = self.state.write()
            && let Some(server) = state.servers.get_mut(&id)
        {
            server.spec.pipelines = bindings;
        }
        let spec = entry.spec.clone();
        let persistence = self.persistence.clone();
        let saved = tokio::task::spawn_blocking(move || persistence.save_spec(&spec)).await;
        if let Ok(Err(error)) = saved {
            self.warn(format!("could not save the pipelines: {error}"));
        }
        (self.notify)();
    }
}

fn next_run_id() -> u64 {
    Utc::now().timestamp_millis().max(0) as u64
}
