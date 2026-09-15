use std::time::Duration;

use asiba_config::AiConfig;
use asiba_core::{
    ActionRecord, ActionRequest, AuditReport, AuditScope, Availability, Credentials, Incident,
    Intervals, ModuleId, QueryRequest, QueryResponse, ServerId, ServerSpec, Snapshot,
    TransportError,
};
use asiba_transport::HostKeyPolicy;
use chrono::{DateTime, Utc};

use crate::alerts::AlertSettings;

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
    Perform {
        server: ServerId,
        module: ModuleId,
        request: ActionRequest,
    },
    AcknowledgeAlert(u64),
    MuteAlert {
        id: u64,
        until: DateTime<Utc>,
    },
    SetAlertSettings(AlertSettings),
    SetIntervals(Intervals),
    Audit {
        server: ServerId,
        scope: AuditScope,
        is_auto: bool,
    },
    SetAiConfig(AiConfig),
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
    ActionFinished(ActionRecord),
    IncidentsOpened(Vec<Incident>),
    AuditFinished(AuditReport),
    ServerSaved(ServerId),
    ServerRemoved(ServerId),
    Warning(String),
}
