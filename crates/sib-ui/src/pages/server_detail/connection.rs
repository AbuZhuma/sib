use egui::{RichText, Ui};
use sib_core::{ConnectionStatus, LocationSource, ServerState};

use crate::components::status_label;
use crate::format;
use crate::pages::Action;
use crate::text;
use crate::theme::Palette;

pub fn show(ui: &mut Ui, server: &ServerState) -> Option<Action> {
    let p = Palette::current(ui.ctx());
    status_label(ui, &server.connection);
    match &server.connection {
        ConnectionStatus::Online { since } => {
            ui.label(format!(
                "{} {}",
                text::DETAIL_ONLINE_SINCE,
                format::date_time(*since)
            ));
            ping_line(ui, server);
            location_line(ui, server);
            None
        }
        ConnectionStatus::Offline { reason, retry_at } => {
            ui.label(RichText::new(reason).color(p.critical));
            let seconds = format::seconds_until(*retry_at);
            ui.label(format!("{} {seconds} s", text::DETAIL_RETRY_AT));
            None
        }
        ConnectionStatus::UntrustedHostKey {
            fingerprint,
            changed,
        } => untrusted_key(ui, server, fingerprint, *changed),
        ConnectionStatus::Connecting => None,
    }
}

fn untrusted_key(
    ui: &mut Ui,
    server: &ServerState,
    fingerprint: &str,
    changed: bool,
) -> Option<Action> {
    let p = Palette::current(ui.ctx());
    let warning = if changed {
        text::TEST_CHANGED_KEY
    } else {
        text::TEST_UNKNOWN_KEY
    };
    ui.label(RichText::new(warning).color(p.warning));
    ui.monospace(fingerprint);
    ui.button(text::BTN_TRUST_KEY_SERVER)
        .clicked()
        .then(|| Action::TrustHostKey {
            server: server.spec.id.clone(),
            fingerprint: fingerprint.to_owned(),
        })
}

fn ping_line(ui: &mut Ui, server: &ServerState) {
    let p = Palette::current(ui.ctx());
    let Some(ping) = &server.ping else {
        return;
    };
    let label = if ping.is_jump_host {
        text::NET_PING_JUMP
    } else {
        text::NET_PING
    };
    match ping.rtt_ms {
        Some(rtt) => ui.monospace(format!("{label} {rtt:.1} ms")),
        None => {
            ui.label(RichText::new(format!("{label} {}", text::NET_PING_LOST)).color(p.warning))
        }
    };
}

fn location_line(ui: &mut Ui, server: &ServerState) {
    let p = Palette::current(ui.ctx());
    let Some(location) = &server.location else {
        ui.label(
            RichText::new(format!(
                "{} {}",
                text::DETAIL_LOCATION,
                text::DETAIL_LOCATION_PENDING
            ))
            .color(p.text_muted),
        );
        return;
    };
    let mut parts = vec![location.label.clone()];
    if !location.provider.is_empty() {
        parts.push(location.provider.clone());
    }
    parts.push(format!("{:.3}, {:.3}", location.lat, location.lon));
    if location.source == LocationSource::Manual {
        parts.push(text::DETAIL_LOCATION_MANUAL.to_owned());
    }
    let joined = parts
        .into_iter()
        .filter(|s| !s.is_empty())
        .collect::<Vec<_>>()
        .join(" · ");
    ui.horizontal(|ui| {
        ui.label(RichText::new(text::DETAIL_LOCATION).color(p.text_secondary));
        ui.label(joined);
    });
}
