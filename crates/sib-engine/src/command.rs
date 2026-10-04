use std::time::Duration;

use chrono::{DateTime, Utc};
use sib_config::AiConfig;
use std::collections::BTreeMap;

use sib_core::{
    ActionRecord, ActionRequest, AuditReport, AuditScope, AuditTarget, Availability,
    CheckOverrides, Credentials, CustomCheck, IgnoredIncident, Incident, Intervals, ModuleId,
    Pipeline, PipelineBinding, QueryRequest, QueryResponse, Retention, ServerId, ServerSpec,
    Snapshot, TransportError,
};
use sib_transport::HostKeyPolicy;

use crate::alerts::AlertSettings;

#[derive(Debug)]
pub enum Command {
    AddServer {
        spec: ServerSpec,
        credentials: Credentials,
    },
    UpdateServer {
        previous: ServerId,
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
    Backfill {
        server: ServerId,
        module: ModuleId,
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
    SetRetention(Retention),
    SetGeolocation(bool),
    SetServerChecks {
        server: ServerId,
        checks: Vec<CustomCheck>,
        overrides: CheckOverrides,
    },
    Audit {
        target: AuditTarget,
        scope: AuditScope,
        is_auto: bool,
    },
    SetAiConfig(AiConfig),
    CancelAudit(u64),
    SetIgnoredIncidents(Vec<IgnoredIncident>),
    RunPipeline {
        server: ServerId,
        pipeline: Pipeline,
        values: BTreeMap<String, String>,
    },
    CancelPipeline(u64),
    SetServerPipelines {
        server: ServerId,
        bindings: Vec<PipelineBinding>,
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
    ActionFinished(ActionRecord),
    IncidentsOpened(Vec<Incident>),
    AuditFinished(AuditReport),
    ServerSaved(ServerId),
    ServerRemoved(ServerId),
    PipelineFinished {
        run_id: u64,
        server: ServerId,
        name: String,
        is_success: bool,
    },
    Warning(String),
}
