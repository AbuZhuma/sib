mod common;
pub mod system;

use asiba_core::ModuleRegistry;

pub fn default_registry() -> ModuleRegistry {
    ModuleRegistry::new().register(system::SystemModule)
}
