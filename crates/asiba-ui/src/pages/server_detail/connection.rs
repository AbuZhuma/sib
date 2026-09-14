use asiba_core::{ConnectionStatus, ServerState};
use egui::{RichText, Ui};

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
            None
        }
        ConnectionStatus::Offline { reason, retry_at } => {
            ui.label(RichText::new(reason).color(p.critical));
            let seconds = format::seconds_until(*retry_at);
            ui.label(format!("{} {seconds} с", text::DETAIL_RETRY_AT));
            None
        }
        ConnectionStatus::UntrustedHostKey {
            fingerprint,
            changed,
        } => {
            let warning = if *changed {
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
                    fingerprint: fingerprint.clone(),
                })
        }
        ConnectionStatus::Connecting => None,
    }
}

fn ping_line(ui: &mut Ui, server: &ServerState) {
    let p = Palette::current(ui.ctx());
    let Some(ping) = &server.ping else {
        return;
    };
    match ping.rtt_ms {
        Some(rtt) => ui.monospace(format!("{} {rtt:.1} ms", text::NET_PING)),
        None => ui.label(
            RichText::new(format!("{} {}", text::NET_PING, text::NET_PING_LOST)).color(p.warning),
        ),
    };
}
