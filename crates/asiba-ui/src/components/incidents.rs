use asiba_core::{Incident, IncidentKind};
use chrono::Utc;
use egui::{RichText, Ui};

use super::{badge, severity_color};
use crate::format;
use crate::text;
use crate::theme::Palette;

fn kind_label(kind: IncidentKind) -> &'static str {
    match kind {
        IncidentKind::Alert => text::INCIDENT_ALERT,
        IncidentKind::Anomaly => text::INCIDENT_ANOMALY,
        IncidentKind::BruteForce => text::INCIDENT_BRUTE_FORCE,
        IncidentKind::SecurityCheck => text::INCIDENT_SECURITY,
        IncidentKind::UnitFailed => text::INCIDENT_UNIT,
        IncidentKind::ContainerDown => text::INCIDENT_CONTAINER,
        IncidentKind::DeployFailed => text::INCIDENT_DEPLOY,
        IncidentKind::DiskFull => text::INCIDENT_DISK,
        IncidentKind::Memory => text::INCIDENT_MEMORY,
        IncidentKind::Updates => text::INCIDENT_UPDATES,
        IncidentKind::ModuleError => text::INCIDENT_MODULE,
        IncidentKind::Clock => text::INCIDENT_CLOCK,
    }
}

pub fn incident_line(ui: &mut Ui, incident: &Incident, show_server: bool) -> bool {
    let p = Palette::current(ui.ctx());
    let mut clicked = false;
    ui.horizontal_wrapped(|ui| {
        badge(
            ui,
            kind_label(incident.kind),
            severity_color(&p, incident.severity),
        );
        if show_server && ui.link(incident.server.as_str()).clicked() {
            clicked = true;
        }
        ui.label(&incident.summary);
        let age =
            format::duration_short((Utc::now() - incident.started_at).num_seconds().max(0) as f64);
        ui.label(RichText::new(age).small().color(p.text_muted));
    });
    clicked
}
