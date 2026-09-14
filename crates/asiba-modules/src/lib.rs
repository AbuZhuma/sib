mod common;
pub mod cpu;
pub mod disk;
pub mod memory;
pub mod network;
pub mod processes;
pub mod system;

use asiba_core::ModuleRegistry;

pub fn default_registry() -> ModuleRegistry {
    ModuleRegistry::new()
        .register(system::SystemModule)
        .register(cpu::CpuModule)
        .register(memory::MemoryModule)
        .register(disk::DiskModule)
        .register(network::NetworkModule)
        .register(processes::ProcessesModule)
}
