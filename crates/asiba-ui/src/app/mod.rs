mod actions;
mod confirm;
mod dialogs;

use std::sync::Arc;
use std::time::Duration;

use asiba_config::{AppConfig, Paths};
use asiba_core::{ServerId, SharedState};
use asiba_engine::{EngineEvent, EngineHandle, RepaintNotifier};
use egui::{CentralPanel, Frame, Margin, Panel};

use crate::devtools::{self, ScreenshotOnStart};
use crate::modules::{self, ModuleView};
use crate::pages::inspector::Inspector;
use crate::pages::server_detail::{self, DetailContext};
use crate::pages::settings::SettingsContext;
use crate::pages::{self, Action, Page, server_form::ServerForm};
use crate::shell::{Notice, sidebar, statusbar};
use crate::text;
use crate::theme::{self, GAP, SIDEBAR_WIDTH, STATUSBAR_HEIGHT};

const REPAINT_INTERVAL: Duration = Duration::from_secs(1);
const WINDOW_SIZE: [f32; 2] = [1280.0, 800.0];
const MIN_WINDOW_SIZE: [f32; 2] = [900.0, 600.0];

pub struct AppDeps {
    pub state: SharedState,
    pub paths: Paths,
    pub config: AppConfig,
}

pub type EngineFactory = Box<dyn FnOnce(RepaintNotifier) -> EngineHandle + Send>;

pub fn run(deps: AppDeps, engine_factory: EngineFactory) -> eframe::Result<()> {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title(text::APP_NAME)
            .with_inner_size(WINDOW_SIZE)
            .with_min_inner_size(MIN_WINDOW_SIZE),
        ..Default::default()
    };
    eframe::run_native(
        text::APP_NAME,
        options,
        Box::new(move |cc| {
            theme::install(&cc.egui_ctx, deps.config.theme);
            let ctx = cc.egui_ctx.clone();
            let notifier: RepaintNotifier = Arc::new(move || ctx.request_repaint());
            let engine = engine_factory(notifier);
            let mut app = AsibaApp::new(deps, engine);
            if let Some((page, tab)) = devtools::start_page() {
                app.page = page;
                if let Some(tab) = tab {
                    server_detail::select_tab(&cc.egui_ctx, tab);
                }
            }
            Ok(Box::new(app))
        }),
    )
}

pub struct AsibaApp {
    engine: EngineHandle,
    state: SharedState,
    paths: Paths,
    config: AppConfig,
    page: Page,
    form: Option<ServerForm>,
    delete_dialog: Option<dialogs::DeleteDialog>,
    confirm_dialog: Option<confirm::ConfirmDialog>,
    notices: Vec<Notice>,
    views: Vec<Box<dyn ModuleView>>,
    screenshot: Option<ScreenshotOnStart>,
    inspector: Option<Inspector>,
    next_query_token: u64,
}

impl AsibaApp {
    fn new(deps: AppDeps, engine: EngineHandle) -> Self {
        Self {
            engine,
            state: deps.state,
            paths: deps.paths,
            config: deps.config,
            page: Page::Overview,
            form: None,
            delete_dialog: None,
            confirm_dialog: None,
            notices: Vec::new(),
            views: modules::all(),
            screenshot: ScreenshotOnStart::from_env(),
            inspector: None,
            next_query_token: 1,
        }
    }

    fn drain_engine_events(&mut self) {
        for event in self.engine.poll_events() {
            match event {
                EngineEvent::TestFinished(report) => {
                    if let Some(form) = &mut self.form {
                        form.accept_report(report);
                    }
                }
                EngineEvent::QueryFinished { token, result } => {
                    if let Some(inspector) = &mut self.inspector {
                        inspector.accept(token, result.map(|r| (r.title, r.text)));
                    }
                }
                EngineEvent::ActionFinished(record) => {
                    let outcome = if record.is_success {
                        text::ACTION_DONE
                    } else {
                        text::ACTION_FAILED
                    };
                    let message = format!(
                        "{} {}: {outcome} — {}",
                        record.kind, record.target, record.message
                    );
                    self.notices.push(Notice::new(message));
                }
                EngineEvent::ServerSaved(_) | EngineEvent::ServerRemoved(_) => {}
                EngineEvent::Warning(message) => self.notices.push(Notice::new(message)),
            }
        }
        self.notices.retain(|n| !n.is_expired());
    }

    fn central(&mut self, ui: &mut egui::Ui) -> Option<Action> {
        let Ok(state) = self.state.read() else {
            return None;
        };
        match self.page.clone() {
            Page::Overview => pages::overview::show(ui, &state),
            Page::Servers => pages::servers::show(ui, &state),
            Page::ServerDetail(id) => match state.servers.get(&id) {
                Some(server) => {
                    let detail = DetailContext {
                        server,
                        views: &self.views,
                        inspector: self.inspector.as_ref(),
                    };
                    pages::server_detail::show(ui, &detail)
                }
                None => Some(Action::Navigate(Page::Servers)),
            },
            Page::ServerForm => {
                let existing: Vec<ServerId> = state.servers.keys().cloned().collect();
                match &mut self.form {
                    Some(form) => form.show(ui, &existing, &self.views),
                    None => Some(Action::Navigate(Page::Servers)),
                }
            }
            Page::Alerts => pages::alerts::show(ui, &state),
            Page::Map => {
                pages::map::show(ui);
                None
            }
            Page::Settings => pages::settings::show(
                ui,
                &SettingsContext {
                    paths: &self.paths,
                    config: &self.config,
                    state: &state,
                },
            ),
        }
    }
}

impl eframe::App for AsibaApp {
    fn ui(&mut self, root: &mut egui::Ui, _frame: &mut eframe::Frame) {
        let ctx = root.ctx().clone();
        ctx.request_repaint_after(REPAINT_INTERVAL);
        if let Some(screenshot) = &mut self.screenshot {
            screenshot.tick(&ctx);
        }
        self.drain_engine_events();
        let palette = theme::Palette::current(&ctx);
        let mut action = None;
        Panel::top("statusbar")
            .exact_size(STATUSBAR_HEIGHT)
            .resizable(false)
            .frame(
                Frame::new()
                    .fill(palette.bg_panel)
                    .inner_margin(Margin::symmetric(GAP as i8, 0)),
            )
            .show(root, |ui| {
                if let Ok(state) = self.state.read() {
                    statusbar(ui, &state, &self.notices);
                }
            });
        Panel::left("sidebar")
            .exact_size(SIDEBAR_WIDTH)
            .resizable(false)
            .frame(
                Frame::new()
                    .fill(palette.bg_window)
                    .inner_margin(Margin::same(GAP as i8)),
            )
            .show(root, |ui| {
                if let Ok(state) = self.state.read() {
                    action = sidebar(ui, &self.page, &state);
                }
            });
        CentralPanel::default_margins()
            .frame(
                Frame::new()
                    .fill(palette.bg_window)
                    .inner_margin(Margin::same(GAP as i8)),
            )
            .show(root, |ui| {
                if let Some(next) = self.central(ui) {
                    action = Some(next);
                }
            });
        self.delete_modal(&ctx);
        self.confirm_modal(&ctx);
        if let Some(action) = action {
            self.apply(action, &ctx);
        }
    }
}
