use std::time::Duration;

use asiba_core::{
    Availability, Credentials, ModuleId, QueryRequest, QueryResponse, ServerId, ServerSpec,
    Snapshot, TransportError,
};
use asiba_transport::HostKeyPolicy;

#[derive(Debug)]
pub enum Command {
    AddServer {
        spec: ServerSpec,
        credentials: Credentials,
    },
    UpdateServer {
        spec: ServerSpec,
        credentials: Credentials,
    },
    RemoveServer(ServerId),
    Reconnect(ServerId),
    TrustHostKey {
        server: ServerId,
        fingerprint: String,
    },
    TestConnection(TestRequest),
    Query {
        token: u64,
        server: ServerId,
        module: ModuleId,
        request: QueryRequest,
    },
}

#[derive(Debug, Clone)]
pub struct TestRequest {
    pub token: u64,
    pub spec: ServerSpec,
    pub credentials: Credentials,
    pub policy: HostKeyPolicy,
    pub probe_module: ModuleId,
}

#[derive(Debug, Clone)]
pub struct ModuleDetection {
    pub id: ModuleId,
    pub title: &'static str,
    pub availability: Availability,
}

#[derive(Debug, Clone)]
pub struct TestReport {
    pub token: u64,
    pub elapsed: Duration,
    pub result: Result<TestSuccess, TransportError>,
}

#[derive(Debug, Clone)]
pub struct TestSuccess {
    pub modules: Vec<ModuleDetection>,
    pub probe: Option<(ModuleId, Snapshot)>,
}

#[derive(Debug, Clone)]
pub enum EngineEvent {
    TestFinished(TestReport),
    QueryFinished {
        token: u64,
        result: Result<QueryResponse, String>,
    },
    ServerSaved(ServerId),
    ServerRemoved(ServerId),
    Warning(String),
}
