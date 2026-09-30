pub mod ai_settings;
pub mod alert_rules;
pub mod alerts;
pub mod collection_settings;
pub mod inspector;
pub mod journal;
pub mod map;
pub mod overview;
pub mod server_detail;
pub mod server_form;
pub mod servers;
pub mod settings;

use chrono::{DateTime, Utc};
use sib_config::{AiConfig, ThemeChoice};
use sib_core::{
    ActionRequest, ActionSpec, AlertRule, AuditScope, AuditTarget, CheckOverrides, Credentials,
    CustomCheck, IgnoredIncident, Intervals, ModuleId, QueryRequest, Retention, ServerId,
    ServerSpec,
};
use sib_engine::TestRequest;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Page {
    Overview,
    Servers,
    ServerDetail(ServerId),
    ServerForm,
    Alerts,
    Map,
    Settings,
}

pub enum Action {
    Navigate(Page),
    OpenForm(Option<ServerId>),
    SaveServer {
        spec: ServerSpec,
        credentials: Credentials,
        previous: Option<ServerId>,
    },
    TestConnection(TestRequest),
    Reconnect(ServerId),
    TrustHostKey {
        server: ServerId,
        fingerprint: String,
    },
    AskDelete(ServerId),
    OpenServerFile(ServerId),
    OpenTerminal(ServerId),
    SetTheme(ThemeChoice),
    SetGeolocation(bool),
    Query {
        server: ServerId,
        module: ModuleId,
        request: QueryRequest,
    },
    Backfill {
        server: ServerId,
        module: ModuleId,
    },
    AskPerform {
        server: ServerId,
        spec: ActionSpec,
        request: ActionRequest,
    },
    CloseInspector,
    FilesQuery {
        server: ServerId,
        request: QueryRequest,
    },
    FilesRefresh {
        server: ServerId,
        path: String,
    },
    FilesClearSearch {
        server: ServerId,
    },
    AcknowledgeAlert(u64),
    MuteAlert {
        id: u64,
        until: DateTime<Utc>,
    },
    SaveAlertSettings {
        rules: Vec<AlertRule>,
        desktop_notifications: bool,
    },
    SaveCollection {
        intervals: Intervals,
        retention: Retention,
    },
    SaveServerChecks {
        server: ServerId,
        checks: Vec<CustomCheck>,
        overrides: CheckOverrides,
    },
    SaveAiConfig(AiConfig),
    Audit {
        target: AuditTarget,
        scope: AuditScope,
    },
    CancelAudit(u64),
    IgnoreIncident(IgnoredIncident),
    RestoreIncident(IgnoredIncident),
}
