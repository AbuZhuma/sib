mod deploy_section;
mod notes;
mod render;
mod sections;

pub use notes::merge_notes;
pub use render::{render, strip_timestamp};

use asiba_core::ModuleId;
use asiba_modules::{deploy, docker, ports, projects, services, system};

pub const TRIGGER_MODULES: [ModuleId; 6] = [
    system::ID,
    deploy::ID,
    projects::ID,
    services::ID,
    docker::ID,
    ports::ID,
];

pub fn is_trigger(module: ModuleId) -> bool {
    TRIGGER_MODULES.contains(&module)
}

pub fn has_required_data(server: &asiba_core::ServerState) -> bool {
    server.snapshot(system::ID).is_some()
}
