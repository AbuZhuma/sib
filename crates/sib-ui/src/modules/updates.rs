use egui::{RichText, Ui};
use sib_core::{ModuleId, ServerState};
use sib_modules::updates::{self, UpdatesSnapshot};

use super::{ModuleView, Tab, ViewShared};
use crate::text;
use crate::theme::Palette;

const SHOWN_PACKAGES: usize = 8;

pub struct UpdatesView;

impl ModuleView for UpdatesView {
    fn id(&self) -> ModuleId {
        updates::ID
    }

    fn title(&self) -> &'static str {
        text::MODULE_UPDATES
    }

    fn tab(&self) -> Tab {
        Tab::Summary
    }

    fn summary(&self, ui: &mut Ui, server: &ServerState, _shared: &ViewShared) {
        let p = Palette::current(ui.ctx());
        let Some(snapshot) = server.data::<UpdatesSnapshot>(updates::ID) else {
            return;
        };
        ui.horizontal_wrapped(|ui| {
            ui.monospace(RichText::new(snapshot.manager.label()).color(p.text_secondary));
            if snapshot.pending == 0 {
                ui.label(RichText::new(text::UPD_NONE).color(p.ok));
            } else {
                ui.monospace(format!("{} {}", snapshot.pending, text::UPD_PENDING));
            }
            if snapshot.security > 0 {
                ui.label(
                    RichText::new(format!("{} {}", snapshot.security, text::UPD_SECURITY))
                        .monospace()
                        .color(p.warning),
                );
            }
        });
        if snapshot.reboot_required {
            ui.label(RichText::new(text::UPD_REBOOT).color(p.warning));
        }
        if !snapshot.packages.is_empty() {
            ui.label(
                RichText::new(packages_line(&snapshot.packages))
                    .small()
                    .color(p.text_muted),
            );
        }
    }
}

fn packages_line(packages: &[String]) -> String {
    let shown: Vec<&str> = packages
        .iter()
        .take(SHOWN_PACKAGES)
        .map(String::as_str)
        .collect();
    let more = packages.len().saturating_sub(SHOWN_PACKAGES);
    let suffix = if more > 0 {
        format!(" +{more}")
    } else {
        String::new()
    };
    format!("{}{suffix}", shown.join(", "))
}
