mod anomalies;
mod cpu;
mod deploy;
mod disk;
mod docker;
mod gpu;
mod logs;
mod memory;
mod network;
mod ports;
mod processes;
mod security;
mod services;
mod system;
mod updates;
mod users;

use std::collections::BTreeMap;

use asiba_core::{ActionRequest, ActionSpec, ModuleId, QueryRequest, ServerState, Snapshot};
use egui::Ui;

use crate::text;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Tab {
    Summary,
    Processes,
    Resources,
    Network,
    Ports,
    Docker,
    Services,
    Logs,
    Users,
    Security,
    Anomalies,
    Deploy,
    Gpu,
    Audit,
}

impl Tab {
    pub const ALL: [Tab; 14] = [
        Tab::Summary,
        Tab::Processes,
        Tab::Resources,
        Tab::Network,
        Tab::Ports,
        Tab::Docker,
        Tab::Services,
        Tab::Logs,
        Tab::Users,
        Tab::Security,
        Tab::Anomalies,
        Tab::Deploy,
        Tab::Gpu,
        Tab::Audit,
    ];

    pub fn from_key(key: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|tab| tab.key() == key)
    }

    pub fn key(self) -> &'static str {
        match self {
            Tab::Summary => "summary",
            Tab::Processes => "processes",
            Tab::Resources => "resources",
            Tab::Network => "network",
            Tab::Ports => "ports",
            Tab::Docker => "docker",
            Tab::Services => "services",
            Tab::Logs => "logs",
            Tab::Users => "users",
            Tab::Security => "security",
            Tab::Anomalies => "anomalies",
            Tab::Deploy => "deploy",
            Tab::Gpu => "gpu",
            Tab::Audit => "audit",
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Tab::Summary => text::TAB_SUMMARY,
            Tab::Processes => text::TAB_PROCESSES,
            Tab::Resources => text::TAB_RESOURCES,
            Tab::Network => text::TAB_NETWORK,
            Tab::Ports => text::TAB_PORTS,
            Tab::Docker => text::TAB_DOCKER,
            Tab::Services => text::TAB_SERVICES,
            Tab::Logs => text::TAB_LOGS,
            Tab::Users => text::TAB_USERS,
            Tab::Security => text::TAB_SECURITY,
            Tab::Anomalies => text::TAB_ANOMALIES,
            Tab::Deploy => text::TAB_DEPLOY,
            Tab::Gpu => text::TAB_GPU,
            Tab::Audit => text::AUDIT_TITLE,
        }
    }
}

pub enum ViewAction {
    Query(QueryRequest),
    Backfill,
    Act {
        spec: ActionSpec,
        request: ActionRequest,
    },
}

impl ViewAction {
    pub fn act(spec: ActionSpec, target: &str) -> Self {
        Self::Act {
            spec,
            request: ActionRequest::new(spec.kind, target),
        }
    }
}

pub fn action_button(
    ui: &mut Ui,
    label: &str,
    spec: ActionSpec,
    target: &str,
) -> Option<ViewAction> {
    ui.small_button(label)
        .clicked()
        .then(|| ViewAction::act(spec, target))
}

pub struct ViewShared<'a> {
    pub countries: &'a BTreeMap<String, String>,
}

impl ViewShared<'_> {
    pub fn country(&self, ip: &str) -> &str {
        self.countries.get(ip).map(String::as_str).unwrap_or("")
    }
}

pub trait ModuleView: Send + Sync {
    fn id(&self) -> ModuleId;

    fn title(&self) -> &'static str;

    fn tab(&self) -> Tab;

    fn summary(&self, ui: &mut Ui, server: &ServerState);

    fn page(&self, ui: &mut Ui, server: &ServerState, _shared: &ViewShared) -> Option<ViewAction> {
        self.summary(ui, server);
        None
    }

    fn preview(&self, _ui: &mut Ui, _snapshot: &Snapshot) {}
}

pub fn all() -> Vec<Box<dyn ModuleView>> {
    vec![
        Box::new(system::SystemView),
        Box::new(cpu::CpuView),
        Box::new(memory::MemoryView),
        Box::new(disk::DiskView),
        Box::new(network::NetworkView),
        Box::new(processes::ProcessesView),
        Box::new(ports::PortsView),
        Box::new(docker::DockerView),
        Box::new(services::ServicesView),
        Box::new(logs::LogsView),
        Box::new(users::UsersView),
        Box::new(updates::UpdatesView),
        Box::new(security::SecurityView),
        Box::new(anomalies::AnomaliesView),
        Box::new(deploy::DeployView),
        Box::new(gpu::GpuView),
    ]
}

pub use anomalies::attack_badge;
pub use deploy::timeline as deploy_timeline;

pub fn short_label(id: ModuleId) -> &'static str {
    match id.0 {
        "system" => "sys",
        "cpu" => "cpu",
        "memory" => "mem",
        "disk" => "dsk",
        "network" => "net",
        "processes" => "prc",
        "services" => "svc",
        "docker" => "dkr",
        "ports" => "prt",
        "logs" => "log",
        "users" => "usr",
        "updates" => "upd",
        "security" => "sec",
        "anomalies" => "anm",
        "deploy" => "dpl",
        "gpu" => "gpu",
        other => other,
    }
}

pub fn has_data(server: &ServerState, id: ModuleId) -> bool {
    server.snapshot(id).is_some()
}
