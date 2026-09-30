use sib_core::{AppState, ModuleData, ModuleId, ServerState};

pub struct AuditContext<'a> {
    pub server: &'a ServerState,
    pub state: &'a AppState,
}

impl AuditContext<'_> {
    pub fn data<T: ModuleData>(&self, module: ModuleId) -> Option<&T> {
        self.server.data::<T>(module)
    }
}

pub fn list_or<T: AsRef<str>>(items: &[T], empty: &str) -> String {
    if items.is_empty() {
        return empty.to_owned();
    }
    items
        .iter()
        .map(AsRef::as_ref)
        .collect::<Vec<_>>()
        .join(", ")
}
