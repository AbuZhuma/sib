pub mod alerts;
pub mod inspector;
pub mod map;
pub mod overview;
pub mod server_detail;
pub mod server_form;
pub mod servers;
pub mod settings;

use asiba_config::ThemeChoice;
use asiba_core::{Credentials, ModuleId, QueryRequest, ServerId, ServerSpec};
use asiba_engine::TestRequest;

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
    SetTheme(ThemeChoice),
    Query {
        server: ServerId,
        module: ModuleId,
        request: QueryRequest,
    },
    CloseInspector,
}
