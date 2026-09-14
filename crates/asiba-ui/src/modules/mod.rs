mod cpu;
mod disk;
mod memory;
mod network;
mod processes;
mod system;

use asiba_core::{ModuleId, ServerState, Snapshot};
use egui::Ui;

use crate::text;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Tab {
    Summary,
    Resources,
    Network,
    Processes,
}

impl Tab {
    pub const ALL: [Tab; 4] = [Tab::Summary, Tab::Resources, Tab::Network, Tab::Processes];

    pub fn label(self) -> &'static str {
        match self {
            Tab::Summary => text::TAB_SUMMARY,
            Tab::Resources => text::TAB_RESOURCES,
            Tab::Network => text::TAB_NETWORK,
            Tab::Processes => text::TAB_PROCESSES,
        }
    }
}

pub trait ModuleView: Send + Sync {
    fn id(&self) -> ModuleId;

    fn title(&self) -> &'static str;

    fn tab(&self) -> Tab;

    fn summary(&self, ui: &mut Ui, server: &ServerState);

    fn page(&self, ui: &mut Ui, server: &ServerState) {
        self.summary(ui, server);
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
    ]
}

pub fn has_data(server: &ServerState, id: ModuleId) -> bool {
    server.snapshot(id).is_some()
}
