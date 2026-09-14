pub mod anomalies;
mod common;
pub mod cpu;
pub mod disk;
pub mod docker;
pub mod logs;
pub mod memory;
pub mod network;
pub mod ports;
pub mod processes;
pub mod projects;
pub mod security;
pub mod services;
pub mod system;
pub mod updates;
pub mod users;

use asiba_core::ModuleRegistry;

pub fn default_registry() -> ModuleRegistry {
    ModuleRegistry::new()
        .register(system::SystemModule)
        .register(cpu::CpuModule)
        .register(memory::MemoryModule)
        .register(disk::DiskModule)
        .register(network::NetworkModule)
        .register(processes::ProcessesModule)
        .register(services::ServicesModule)
        .register(docker::DockerModule)
        .register(ports::PortsModule)
        .register(logs::LogsModule)
        .register(users::UsersModule)
        .register(updates::UpdatesModule)
        .register(projects::ProjectsModule)
        .register(security::SecurityModule)
        .register(anomalies::AnomaliesModule)
}
