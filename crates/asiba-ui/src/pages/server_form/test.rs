use std::time::Instant;

use asiba_core::{Availability, TransportError};
use asiba_engine::{TestReport, TestSuccess};
use asiba_transport::HostKeyPolicy;
use egui::{RichText, Ui};

use crate::components::badge;
use crate::modules::ModuleView;
use crate::text;
use crate::theme::Palette;

#[derive(Debug, Default)]
pub enum TestState {
    #[default]
    Idle,
    Running {
        token: u64,
        started: Instant,
    },
    Done(TestReport),
}

impl TestState {
    pub fn is_running(&self) -> bool {
        matches!(self, Self::Running { .. })
    }

    pub fn accept(&mut self, report: TestReport) {
        if matches!(self, Self::Running { token, .. } if *token == report.token) {
            *self = Self::Done(report);
        }
    }

    pub fn untrusted_fingerprint(&self) -> Option<String> {
        let Self::Done(report) = self else {
            return None;
        };
        match &report.result {
            Err(TransportError::UnknownHostKey { fingerprint }) => Some(fingerprint.clone()),
            _ => None,
        }
    }
}

pub enum TestAction {
    Run(HostKeyPolicy),
}

pub fn show(ui: &mut Ui, state: &TestState, views: &[Box<dyn ModuleView>]) -> Option<TestAction> {
    let p = Palette::current(ui.ctx());
    let mut action = None;
    ui.horizontal(|ui| {
        if ui
            .add_enabled(!state.is_running(), egui::Button::new(text::BTN_TEST))
            .clicked()
        {
            action = Some(TestAction::Run(HostKeyPolicy::KnownHostsOnly));
        }
        if let Some(fingerprint) = state.untrusted_fingerprint()
            && ui.button(text::BTN_TRUST_KEY).clicked()
        {
            action = Some(TestAction::Run(HostKeyPolicy::TrustFingerprint(
                fingerprint,
            )));
        }
    });
    match state {
        TestState::Idle => {
            ui.label(RichText::new(text::TEST_IDLE).color(p.text_muted));
        }
        TestState::Running { started, .. } => {
            ui.label(format!(
                "{} {:.0} с",
                text::TEST_RUNNING,
                started.elapsed().as_secs_f32()
            ));
        }
        TestState::Done(report) => report_view(ui, report, views),
    }
    action
}

fn report_view(ui: &mut Ui, report: &TestReport, views: &[Box<dyn ModuleView>]) {
    let p = Palette::current(ui.ctx());
    match &report.result {
        Ok(success) => {
            ui.label(
                RichText::new(format!(
                    "{} ({:.1} с)",
                    text::TEST_OK,
                    report.elapsed.as_secs_f32()
                ))
                .color(p.ok),
            );
            success_view(ui, success, views);
        }
        Err(TransportError::UnknownHostKey { fingerprint }) => {
            ui.label(RichText::new(text::TEST_UNKNOWN_KEY).color(p.warning));
            ui.monospace(fingerprint);
        }
        Err(TransportError::HostKeyChanged { fingerprint }) => {
            ui.label(RichText::new(text::TEST_CHANGED_KEY).color(p.critical));
            ui.monospace(fingerprint);
        }
        Err(error) => {
            ui.label(RichText::new(format!("{}: {error}", text::TEST_FAILED)).color(p.critical));
        }
    }
}

fn success_view(ui: &mut Ui, success: &TestSuccess, views: &[Box<dyn ModuleView>]) {
    let p = Palette::current(ui.ctx());
    if let Some((id, snapshot)) = &success.probe
        && let Some(view) = views.iter().find(|v| v.id() == *id)
    {
        view.preview(ui, snapshot);
    }
    ui.add_space(4.0);
    ui.label(
        RichText::new(text::TEST_MODULES)
            .small()
            .color(p.text_secondary),
    );
    for module in &success.modules {
        ui.horizontal(|ui| {
            ui.monospace(module.id.0);
            match &module.availability {
                Availability::Available => badge(ui, text::AVAIL_AVAILABLE, p.ok),
                Availability::Partial { .. } => badge(ui, text::AVAIL_PARTIAL, p.warning),
                Availability::Unavailable { reason } => {
                    badge(ui, text::AVAIL_UNAVAILABLE, p.offline);
                    ui.label(RichText::new(reason).small().color(p.text_muted));
                }
            }
        });
    }
}
