use std::sync::Arc;

use crate::module::{Module, ModuleId};

#[derive(Default, Clone)]
pub struct ModuleRegistry {
    modules: Vec<Arc<dyn Module>>,
}

impl ModuleRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn register(mut self, module: impl Module + 'static) -> Self {
        self.modules.push(Arc::new(module));
        self
    }

    pub fn all(&self) -> &[Arc<dyn Module>] {
        &self.modules
    }

    pub fn get(&self, id: ModuleId) -> Option<&Arc<dyn Module>> {
        self.modules.iter().find(|module| module.id() == id)
    }

    pub fn len(&self) -> usize {
        self.modules.len()
    }

    pub fn is_empty(&self) -> bool {
        self.modules.is_empty()
    }
}
