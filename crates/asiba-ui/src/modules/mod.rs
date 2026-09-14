mod cpu;
mod disk;
mod docker;
mod logs;
mod memory;
mod network;
mod ports;
mod processes;
mod projects;
mod security;
mod services;
mod system;
mod updates;
mod users;

use asiba_core::{ModuleId, QueryRequest, ServerState, Snapshot};
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
    Projects,
    Logs,
    Users,
    Security,
}

impl Tab {
    pub const ALL: [Tab; 11] = [
        Tab::Summary,
        Tab::Processes,
        Tab::Resources,
        Tab::Network,
        Tab::Ports,
        Tab::Docker,
        Tab::Services,
        Tab::Projects,
        Tab::Logs,
        Tab::Users,
        Tab::Security,
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
            Tab::Projects => "projects",
            Tab::Logs => "logs",
            Tab::Users => "users",
            Tab::Security => "security",
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
            Tab::Projects => text::TAB_PROJECTS,
            Tab::Logs => text::TAB_LOGS,
            Tab::Users => text::TAB_USERS,
            Tab::Security => text::TAB_SECURITY,
        }
    }
}

pub enum ViewAction {
    Query(QueryRequest),
}

pub trait ModuleView: Send + Sync {
    fn id(&self) -> ModuleId;

    fn title(&self) -> &'static str;

    fn tab(&self) -> Tab;

    fn summary(&self, ui: &mut Ui, server: &ServerState);

    fn page(&self, ui: &mut Ui, server: &ServerState) -> Option<ViewAction> {
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
        Box::new(projects::ProjectsView),
        Box::new(logs::LogsView),
        Box::new(users::UsersView),
        Box::new(updates::UpdatesView),
        Box::new(security::SecurityView),
    ]
}

pub fn has_data(server: &ServerState, id: ModuleId) -> bool {
    server.snapshot(id).is_some()
}
