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

use asiba_config::{AiConfig, Retention, SummaryLayout, ThemeChoice};
use asiba_core::{
    ActionRequest, ActionSpec, AlertRule, AuditScope, AuditTarget, Credentials, Intervals,
    ModuleId, QueryRequest, ServerId, ServerSpec,
};
use asiba_engine::TestRequest;
use chrono::{DateTime, Utc};

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
        is_new: bool,
    },
    TestConnection(TestRequest),
    Reconnect(ServerId),
    TrustHostKey {
        server: ServerId,
        fingerprint: String,
    },
    AskDelete(ServerId),
    OpenServerFile(ServerId),
    SaveLayout {
        server: ServerId,
        layout: SummaryLayout,
    },
    OpenTerminal(ServerId),
    SetTheme(ThemeChoice),
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
    SaveAiConfig(AiConfig),
    Audit {
        target: AuditTarget,
        scope: AuditScope,
    },
    CancelAudit(u64),
}
