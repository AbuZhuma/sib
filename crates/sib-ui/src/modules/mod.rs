mod anomalies;
mod checks;
mod cpu;
mod deploy;
mod disk;
mod docker;
mod git;
mod gpu;
mod logs;
mod memory;
mod network;
mod ports;
mod processes;
pub mod security;
mod services;
mod system;
mod updates;
mod users;

use std::collections::BTreeMap;

use egui::Ui;
use sib_core::{
    ActionRequest, ActionSpec, AppState, ModuleId, QueryRequest, ServerState, Snapshot,
};

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
    Git,
    Gpu,
    Checks,
    Files,
    Pipelines,
}

impl Tab {
    pub const ALL: [Tab; 17] = [
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
        Tab::Git,
        Tab::Gpu,
        Tab::Checks,
        Tab::Files,
        Tab::Pipelines,
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
            Tab::Git => "git",
            Tab::Gpu => "gpu",
            Tab::Checks => "checks",
            Tab::Files => "files",
            Tab::Pipelines => "pipelines",
        }
    }

    pub fn for_module(module: &str) -> Option<Self> {
        let tab = match module {
            "system" | "updates" | "connection" | "ping" => Tab::Summary,
            "cpu" | "memory" | "disk" => Tab::Resources,
            "network" => Tab::Network,
            "processes" => Tab::Processes,
            "ports" => Tab::Ports,
            "docker" => Tab::Docker,
            "services" => Tab::Services,
            "logs" => Tab::Logs,
            "users" => Tab::Users,
            "security" => Tab::Security,
            "anomalies" => Tab::Anomalies,
            "deploy" => Tab::Deploy,
            "git" => Tab::Git,
            "gpu" => Tab::Gpu,
            "checks" => Tab::Checks,
            "files" => Tab::Files,
            "pipelines" | "pipeline" => Tab::Pipelines,
            _ => return None,
        };
        Some(tab)
    }

    pub fn section_key(self) -> Option<&'static str> {
        match self {
            Tab::Processes => Some("processes"),
            Tab::Resources | Tab::Network => Some("resources"),
            Tab::Ports => Some("ports"),
            Tab::Docker => Some("docker"),
            Tab::Services => Some("services"),
            Tab::Logs => Some("logs"),
            Tab::Users => Some("users"),
            Tab::Security => Some("security"),
            Tab::Anomalies => Some("anomalies"),
            Tab::Deploy => Some("deploy"),
            Tab::Gpu => Some("gpu"),
            Tab::Summary | Tab::Git | Tab::Files | Tab::Checks | Tab::Pipelines => None,
        }
    }

    pub fn for_metric(metric: &str) -> Self {
        let module = metric.split('.').next().unwrap_or_default();
        Self::for_module(module).unwrap_or(Tab::Summary)
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
            Tab::Git => text::TAB_GIT,
            Tab::Gpu => text::TAB_GPU,
            Tab::Checks => text::TAB_CHECKS,
            Tab::Files => text::TAB_FILES,
            Tab::Pipelines => text::TAB_PIPELINES,
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
    pub state: &'a AppState,
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

    fn has_content(&self, server: &ServerState) -> bool {
        has_data(server, self.id())
    }

    fn summary(&self, ui: &mut Ui, server: &ServerState, shared: &ViewShared);

    fn page(&self, ui: &mut Ui, server: &ServerState, shared: &ViewShared) -> Option<ViewAction> {
        self.summary(ui, server, shared);
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
        Box::new(git::GitView),
        Box::new(gpu::GpuView),
        Box::new(checks::ChecksView),
    ]
}

pub use anomalies::attack_badge;
pub use deploy::timeline as deploy_timeline;
pub use security as security_view;

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
        "git" => "git",
        "gpu" => "gpu",
        "checks" => "chk",
        "files" => "fil",
        other => other,
    }
}

pub fn has_data(server: &ServerState, id: ModuleId) -> bool {
    server.snapshot(id).is_some()
}
