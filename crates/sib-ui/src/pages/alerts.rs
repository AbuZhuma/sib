use chrono::{Duration, Utc};
use egui::{CursorIcon, Label, RichText, Sense, Ui};
use sib_core::{Alert, AppState, IgnoredIncident};

use super::alert_rules::severity_label;
use super::server_detail::select_tab;
use super::{Action, Page};
use crate::components::{Table, badge, kind_label, page_title, panel, scroll, severity_color};
use crate::format;
use crate::modules::Tab;
use crate::text;
use crate::theme::{GAP, Palette};

const HISTORY_SHOWN: usize = 100;

pub fn show(ui: &mut Ui, state: &AppState) -> Option<Action> {
    page_title(ui, text::ALERTS_TITLE);
    let mut action = None;
    scroll::vertical().show(ui, |ui| {
        action = panel(ui, text::ALERTS_ACTIVE, |ui| active_table(ui, state));
        ui.add_space(GAP);
        let history = panel(ui, text::ALERTS_HISTORY, |ui| history_table(ui, state));
        if history.is_some() {
            action = history;
        }
        ui.add_space(GAP);
        let archive = panel(ui, text::INCIDENTS_ARCHIVE, |ui| archive_list(ui, state));
        if archive.is_some() {
            action = archive;
        }
    });
    action
}

fn archive_list(ui: &mut Ui, state: &AppState) -> Option<Action> {
    let p = Palette::current(ui.ctx());
    if state.ignored_incidents.is_empty() {
        ui.label(RichText::new(text::INCIDENTS_ARCHIVE_EMPTY).color(p.text_muted));
        return None;
    }
    let mut action = None;
    for entry in &state.ignored_incidents {
        ui.horizontal_wrapped(|ui| {
            badge(ui, kind_label(entry.kind), p.text_secondary);
            if ui.link(entry.server.as_str()).clicked() {
                action = Some(Action::Navigate(Page::ServerDetail(entry.server.clone())));
            }
            ui.monospace(&entry.subject);
            if ui.small_button(text::INCIDENT_RESTORE).clicked() {
                action = Some(Action::RestoreIncident(IgnoredIncident::clone(entry)));
            }
        });
    }
    action
}

fn active_table(ui: &mut Ui, state: &AppState) -> Option<Action> {
    let p = Palette::current(ui.ctx());
    let now = Utc::now();
    let mut active: Vec<&Alert> = state.active_alerts().collect();
    if active.is_empty() {
        ui.label(RichText::new(text::ALERTS_NONE_ACTIVE).color(p.text_muted));
        return None;
    }
    active.sort_by_key(|a| (std::cmp::Reverse(a.severity), a.started_at));
    let mut action = None;
    Table::new("alerts-active", &ACTIVE_COLUMNS).show(ui, |ui| {
        for alert in active {
            let color = severity_color(&p, alert.severity);
            badge(ui, severity_label(alert.severity), color);
            if ui.link(alert.server.as_str()).clicked() || rule_cell(ui, alert, &p) {
                action = Some(open_details(ui, alert));
            }
            ui.monospace(format!("{:.1}", alert.value));
            ui.monospace(format::clock(alert.started_at));
            ui.monospace(format::duration_short(
                alert.duration(now).num_seconds() as f64
            ));
            if let Some(next) = alert_buttons(ui, alert) {
                action = Some(next);
            }
            ui.end_row();
        }
    });
    action
}

const ACTIVE_COLUMNS: [&str; 7] = [
    text::COL_SEVERITY,
    text::COL_SERVER,
    text::ALERTS_RULE,
    text::ALERTS_VALUE,
    text::ALERTS_STARTED,
    text::ALERTS_DURATION,
    "",
];

fn open_details(ui: &Ui, alert: &Alert) -> Action {
    select_tab(ui.ctx(), Tab::for_metric(&alert.metric));
    Action::Navigate(Page::ServerDetail(alert.server.clone()))
}

fn rule_cell(ui: &mut Ui, alert: &Alert, p: &Palette) -> bool {
    ui.vertical(|ui| {
        let name = clickable(ui, RichText::new(&alert.rule_name));
        let message = RichText::new(&alert.message)
            .small()
            .color(p.text_secondary);
        clickable(ui, message) || name
    })
    .inner
}

fn clickable(ui: &mut Ui, text: RichText) -> bool {
    ui.add(Label::new(text).extend().sense(Sense::click()))
        .on_hover_cursor(CursorIcon::PointingHand)
        .on_hover_text(text::ALERTS_OPEN_HINT)
        .clicked()
}

fn alert_buttons(ui: &mut Ui, alert: &Alert) -> Option<Action> {
    let p = Palette::current(ui.ctx());
    let now = Utc::now();
    let mut action = None;
    ui.horizontal(|ui| {
        if alert.is_muted(now) {
            let until = alert.muted_until.map(format::clock).unwrap_or_default();
            ui.label(RichText::new(format!("{} {until}", text::ALERTS_MUTED)).color(p.text_muted));
            return;
        }
        if alert.acknowledged {
            ui.label(RichText::new(text::ALERTS_ACKED).color(p.text_muted));
        } else if ui.small_button(text::ALERTS_ACK).clicked() {
            action = Some(Action::AcknowledgeAlert(alert.id));
        }
        if ui.small_button(text::ALERTS_MUTE_1H).clicked() {
            action = Some(Action::MuteAlert {
                id: alert.id,
                until: now + Duration::hours(1),
            });
        }
        if ui.small_button(text::ALERTS_MUTE_24H).clicked() {
            action = Some(Action::MuteAlert {
                id: alert.id,
                until: now + Duration::hours(24),
            });
        }
    });
    action
}

fn history_table(ui: &mut Ui, state: &AppState) -> Option<Action> {
    let p = Palette::current(ui.ctx());
    let now = Utc::now();
    let resolved: Vec<&Alert> = state
        .alerts
        .iter()
        .filter(|a| !a.is_active())
        .take(HISTORY_SHOWN)
        .collect();
    if resolved.is_empty() {
        ui.label(RichText::new(text::ALERTS_NONE_HISTORY).color(p.text_muted));
        return None;
    }
    let mut action = None;
    let columns = [
        text::COL_SEVERITY,
        text::COL_SERVER,
        text::ALERTS_RULE,
        text::ALERTS_STARTED,
        text::ALERTS_DURATION,
    ];
    Table::new("alerts-history", &columns).show(ui, |ui| {
        for alert in resolved {
            let color = severity_color(&p, alert.severity);
            badge(ui, severity_label(alert.severity), color);
            if ui.link(alert.server.as_str()).clicked() {
                action = Some(open_details(ui, alert));
            }
            if clickable(ui, RichText::new(&alert.rule_name)) {
                action = Some(open_details(ui, alert));
            }
            ui.monospace(format::date_time(alert.started_at));
            ui.monospace(format::duration_short(
                alert.duration(now).num_seconds() as f64
            ));
            ui.end_row();
        }
    });
    action
}
