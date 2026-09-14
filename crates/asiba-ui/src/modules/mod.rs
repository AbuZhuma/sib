mod system;

use asiba_core::{ModuleId, Snapshot};
use egui::Ui;

pub trait ModuleView: Send + Sync {
    fn id(&self) -> ModuleId;

    fn title(&self) -> &'static str;

    fn summary(&self, ui: &mut Ui, snapshot: &Snapshot);
}

pub fn all() -> Vec<Box<dyn ModuleView>> {
    vec![Box::new(system::SystemView)]
}
