use std::time::Instant;

use asiba_core::{Availability, CollectContext, ModuleRegistry, Transport, TransportError};
use asiba_transport::connect;

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
    let modules = detect_all(transport.as_ref(), registry).await;
    let context = CollectContext {
        previous: None,
        host: request.spec.host.clone(),
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
    transport: &dyn Transport,
    registry: &ModuleRegistry,
) -> Vec<ModuleDetection> {
    let mut detections = Vec::with_capacity(registry.len());
    for module in registry.all() {
        let availability = match module.detect(transport).await {
            Ok(availability) => availability,
            Err(error) => Availability::Unavailable {
                reason: error.to_string(),
            },
        };
        detections.push(ModuleDetection {
            id: module.id(),
            title: module.title(),
            availability,
        });
    }
    detections
}
