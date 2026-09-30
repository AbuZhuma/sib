use chrono::Utc;
use egui::{CursorIcon, Label, RichText, Sense, Ui};
use sib_core::{AppState, AuditScope, Incident, IncidentKind};

use super::{badge, severity_color};
use crate::format;
use crate::modules::Tab;
use crate::text;
use crate::theme::Palette;

pub fn kind_label(kind: IncidentKind) -> &'static str {
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

pub struct IncidentLine<'a> {
    pub incident: &'a Incident,
    pub show_server: bool,
    pub can_audit: bool,
    pub has_report: bool,
}

#[derive(Default)]
pub struct IncidentClick {
    pub open_server: bool,
    pub open_details: bool,
    pub audit: bool,
    pub open_report: bool,
    pub ignore: bool,
}

pub fn incident_tab(state: &AppState, incident: &Incident) -> Tab {
    match incident.kind {
        IncidentKind::Alert => state
            .alerts
            .iter()
            .find(|a| a.server == incident.server && a.rule_id == incident.subject)
            .map(|a| Tab::for_metric(&a.metric))
            .unwrap_or(Tab::Summary),
        IncidentKind::Anomaly => Tab::Anomalies,
        IncidentKind::BruteForce | IncidentKind::SecurityCheck => Tab::Security,
        IncidentKind::UnitFailed => Tab::Services,
        IncidentKind::ContainerDown => Tab::Docker,
        IncidentKind::DeployFailed => Tab::Deploy,
        IncidentKind::DiskFull | IncidentKind::Memory => Tab::Resources,
        IncidentKind::Updates | IncidentKind::ModuleError | IncidentKind::Clock => Tab::Summary,
    }
}

pub fn incident_line(ui: &mut Ui, line: &IncidentLine<'_>) -> IncidentClick {
    let p = Palette::current(ui.ctx());
    let incident = line.incident;
    let mut click = IncidentClick::default();
    ui.horizontal_wrapped(|ui| {
        badge(
            ui,
            kind_label(incident.kind),
            severity_color(&p, incident.severity),
        );
        if line.show_server && ui.link(incident.server.as_str()).clicked() {
            click.open_server = true;
        }
        let summary = ui
            .add(Label::new(&incident.summary).sense(Sense::click()))
            .on_hover_cursor(CursorIcon::PointingHand)
            .on_hover_text(text::INCIDENT_OPEN_HINT);
        if summary.clicked() {
            click.open_details = true;
        }
        let age =
            format::duration_short((Utc::now() - incident.started_at).num_seconds().max(0) as f64);
        ui.label(RichText::new(age).small().color(p.text_muted));
        if line.has_report && ui.small_button(text::AUDIT_REPORT).clicked() {
            click.open_report = true;
        }
        if line.can_audit && ui.small_button(text::AUDIT_INCIDENT).clicked() {
            click.audit = true;
        }
        if ui
            .small_button(text::INCIDENT_IGNORE)
            .on_hover_text(text::INCIDENT_IGNORE_HINT)
            .clicked()
        {
            click.ignore = true;
        }
    });
    click
}

pub fn has_report(state: &AppState, incident: &Incident) -> bool {
    state.audits.iter().any(|a| {
        a.is_for(&incident.server)
            && matches!(&a.scope, AuditScope::Incident { incident_id, .. } if *incident_id == incident.id)
    })
}

pub fn incident_scope(incident: &Incident) -> AuditScope {
    AuditScope::Incident {
        incident_id: incident.id,
        kind: incident.kind,
        subject: incident.subject.clone(),
    }
}
