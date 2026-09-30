use std::sync::Arc;
use std::time::Instant;

use sib_core::{
    Availability, CollectContext, Module, ModuleError, ModuleRegistry, ModuleSettings, ServerSpec,
    Transport, TransportError,
};
use sib_transport::connect;
use tokio::task::JoinSet;

use crate::command::{ModuleDetection, TestReport, TestRequest, TestSuccess};

pub async fn run(request: TestRequest, registry: ModuleRegistry) -> TestReport {
    let started = Instant::now();
    let result = probe(&request, &registry).await;
    TestReport {
        token: request.token,
        elapsed: started.elapsed(),
        result,
    }
}

async fn probe(
    request: &TestRequest,
    registry: &ModuleRegistry,
) -> Result<TestSuccess, TransportError> {
    let transport = connect(&request.spec, &request.credentials, request.policy.clone()).await?;
    let modules = detect_all(&transport, registry, &request.spec).await?;
    let context = CollectContext {
        previous: None,
        host: request.spec.external_host(),
        settings: request.spec.module_settings(request.probe_module.0),
        checks: request.spec.checks.clone(),
    };
    let probe = match registry.get(request.probe_module) {
        Some(module) => module
            .collect(transport.as_ref(), &context)
            .await
            .ok()
            .map(|s| (module.id(), s)),
        None => None,
    };
    Ok(TestSuccess { modules, probe })
}

pub async fn detect_all(
    transport: &Arc<dyn Transport>,
    registry: &ModuleRegistry,
    spec: &ServerSpec,
) -> Result<Vec<ModuleDetection>, TransportError> {
    let mut tasks = JoinSet::new();
    for (index, module) in registry.all().iter().enumerate() {
        let module = Arc::clone(module);
        let transport = Arc::clone(transport);
        let settings = spec.module_settings(module.id().0);
        tasks.spawn(async move {
            let detection = detect_one(module.as_ref(), transport.as_ref(), &settings).await;
            (index, detection)
        });
    }
    let mut detections: Vec<Option<ModuleDetection>> = (0..registry.len()).map(|_| None).collect();
    while let Some(Ok((index, detection))) = tasks.join_next().await {
        detections[index] = Some(detection?);
    }
    Ok(detections.into_iter().flatten().collect())
}

async fn detect_one(
    module: &dyn Module,
    transport: &dyn Transport,
    settings: &ModuleSettings,
) -> Result<ModuleDetection, TransportError> {
    let availability = match module.detect(transport, settings).await {
        Ok(availability) => availability,
        Err(ModuleError::Transport(TransportError::Disconnected(reason))) => {
            return Err(TransportError::Disconnected(reason));
        }
        Err(error) => Availability::Unavailable {
            reason: error.to_string(),
        },
    };
    Ok(ModuleDetection {
        id: module.id(),
        title: module.title(),
        availability,
    })
}

#[cfg(test)]
mod tests {
    use async_trait::async_trait;
    use sib_core::{
        Availability, CollectContext, CommandOutput, Module, ModuleError, ModuleId, ModuleRegistry,
        ModuleSettings, Schedule, Snapshot, SudoMode, Transport,
    };

    use super::*;

    struct DeadTransport;

    #[async_trait]
    impl Transport for DeadTransport {
        async fn exec(&self, _command: &str) -> Result<CommandOutput, TransportError> {
            Err(TransportError::Disconnected("сессия закрыта".to_owned()))
        }

        async fn exec_root(&self, _command: &str) -> Result<CommandOutput, TransportError> {
            Err(TransportError::Disconnected("сессия закрыта".to_owned()))
        }

        fn sudo_mode(&self) -> SudoMode {
            SudoMode::None
        }
    }

    struct Probe;

    #[async_trait]
    impl Module for Probe {
        fn id(&self) -> ModuleId {
            ModuleId("probe")
        }

        fn title(&self) -> &'static str {
            "probe"
        }

        fn schedule(&self) -> Schedule {
            Schedule::Normal
        }

        async fn detect(
            &self,
            transport: &dyn Transport,
            _settings: &ModuleSettings,
        ) -> Result<Availability, ModuleError> {
            transport.exec("true").await?;
            Ok(Availability::Available)
        }

        async fn collect(
            &self,
            _transport: &dyn Transport,
            _context: &CollectContext,
        ) -> Result<Snapshot, ModuleError> {
            Ok(Snapshot::new(()))
        }
    }

    #[tokio::test]
    async fn detect_all_propagates_a_dropped_session() {
        let transport: Arc<dyn Transport> = Arc::new(DeadTransport);
        let registry = ModuleRegistry::new().register(Probe);
        let spec = ServerSpec {
            id: sib_core::ServerId::parse("neo").expect("id"),
            host: "h".into(),
            port: 22,
            user: "u".into(),
            auth: sib_core::AuthMethod::Auto,
            jump: None,
            sudo: SudoMode::None,
            description: Default::default(),
            location: None,
            modules: Default::default(),
            checks: Vec::new(),
            check_overrides: Default::default(),
        };
        let result = detect_all(&transport, &registry, &spec).await;
        assert!(matches!(result, Err(TransportError::Disconnected(_))));
    }
}
