mod actions;
mod confirm;
mod dialogs;
mod dispatch;
mod events;
mod external;

use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;

use asiba_config::{AppConfig, IgnoredStore, LayoutStore, Paths, SecretStore};
use asiba_core::{AppState, ServerId, SharedState};
use asiba_engine::{EngineHandle, RepaintNotifier};
use egui::{CentralPanel, Frame, Margin, Panel};

use crate::components::MapState;
use crate::devtools::{self, ScreenshotOnStart};
use crate::modules::{self, ModuleView, Tab, ViewShared};
use crate::pages::inspector::Inspector;
use crate::pages::server_detail::{self, DetailContext, files::FileBrowser};
use crate::pages::settings::SettingsContext;
use crate::pages::{self, Action, Page, server_form::ServerForm};
use crate::shell::{Notice, StatusContext, sidebar, statusbar};
use crate::text;
use crate::theme::{self, GAP, SIDEBAR_WIDTH, STATUSBAR_HEIGHT};

const REPAINT_INTERVAL: Duration = Duration::from_secs(1);
const WINDOW_SIZE: [f32; 2] = [1280.0, 800.0];
const MIN_WINDOW_SIZE: [f32; 2] = [900.0, 600.0];

pub struct AppDeps {
    pub state: SharedState,
    pub paths: Paths,
    pub config: AppConfig,
    pub secrets: Arc<dyn SecretStore>,
}

pub type EngineFactory = Box<dyn FnOnce(RepaintNotifier) -> EngineHandle + Send>;

pub fn run(deps: AppDeps, engine_factory: EngineFactory) -> eframe::Result<()> {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title(text::APP_NAME)
            .with_inner_size(devtools::window_size().unwrap_or(WINDOW_SIZE))
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
            let mut app = AsibaApp::new(deps, engine, &cc.egui_ctx);
            app.start_page = devtools::start_page();
            Ok(Box::new(app))
        }),
    )
}

pub struct AsibaApp {
    engine: EngineHandle,
    state: SharedState,
    paths: Paths,
    config: AppConfig,
    secrets: Arc<dyn SecretStore>,
    page: Page,
    form: Option<ServerForm>,
    delete_dialog: Option<dialogs::DeleteDialog>,
    confirm_dialog: Option<confirm::ConfirmDialog>,
    notices: Vec<Notice>,
    views: Vec<Box<dyn ModuleView>>,
    screenshot: Option<ScreenshotOnStart>,
    start_page: Option<(Page, Option<Tab>)>,
    map: MapState,
    paused: bool,
    frozen: Option<Arc<AppState>>,
    layouts: LayoutStore,
    ignored: IgnoredStore,
    inspector: Option<Inspector>,
    files: HashMap<ServerId, FileBrowser>,
    next_query_token: u64,
}

impl AsibaApp {
    fn new(deps: AppDeps, engine: EngineHandle, ctx: &egui::Context) -> Self {
        let map = MapState::new(ctx, deps.paths.tiles_cache());
        let layouts = LayoutStore::load(&deps.paths);
        let ignored = IgnoredStore::load(&deps.paths);
        Self {
            map,
            layouts,
            ignored,
            engine,
            state: deps.state,
            paths: deps.paths,
            config: deps.config,
            secrets: deps.secrets,
            page: Page::Overview,
            form: None,
            delete_dialog: None,
            confirm_dialog: None,
            notices: Vec::new(),
            views: modules::all(),
            screenshot: ScreenshotOnStart::from_env(),
            start_page: None,
            paused: false,
            frozen: None,
            inspector: None,
            files: HashMap::new(),
            next_query_token: 1,
        }
    }

    fn apply_start_page(&mut self, ctx: &egui::Context) {
        let Some((page, tab)) = self.start_page.clone() else {
            return;
        };
        let is_ready = match &page {
            Page::ServerDetail(id) => self
                .state
                .read()
                .is_ok_and(|state| state.servers.contains_key(id)),
            _ => true,
        };
        if !is_ready {
            return;
        }
        self.page = page;
        if let Some(tab) = tab {
            server_detail::select_tab(ctx, tab);
        }
        self.start_page = None;
    }

    fn statusbar(&mut self, ui: &mut egui::Ui) {
        let Ok(state) = self.state.read() else {
            return;
        };
        statusbar(
            ui,
            StatusContext {
                state: &state,
                notices: &self.notices,
                paused: &mut self.paused,
            },
        );
    }

    fn sync_pause(&mut self) {
        match (self.paused, self.frozen.is_some()) {
            (true, false) => {
                self.frozen = self.state.read().ok().map(|s| Arc::new(s.clone()));
            }
            (false, true) => self.frozen = None,
            _ => {}
        }
    }

    fn server_detail(&self, ui: &mut egui::Ui, state: &AppState, id: &ServerId) -> Option<Action> {
        let Some(server) = state.servers.get(id) else {
            return Some(Action::Navigate(Page::Servers));
        };
        let layout = self.layouts.for_server(id.as_str());
        let detail = DetailContext {
            server,
            state,
            ai: &self.config.ai,
            views: &self.views,
            inspector: self.inspector.as_ref(),
            files: self.files.get(id),
            layout: &layout,
            shared: ViewShared {
                state,
                countries: &state.ip_countries,
            },
        };
        pages::server_detail::show(ui, &detail)
    }

    fn central(&mut self, ui: &mut egui::Ui) -> Option<Action> {
        let frozen = self.frozen.clone();
        let Ok(guard) = self.state.read() else {
            return None;
        };
        let state: &AppState = frozen.as_deref().unwrap_or(&guard);
        match self.page.clone() {
            Page::Overview => pages::overview::show(
                ui,
                pages::overview::OverviewContext {
                    state,
                    map: &mut self.map,
                    ai_consent: self.config.ai.consent,
                    can_audit: self.config.ai.is_ready(),
                },
            ),
            Page::Servers => pages::servers::show(ui, state),
            Page::ServerDetail(id) => self.server_detail(ui, state, &id),
            Page::ServerForm => {
                let existing: Vec<ServerId> = state.servers.keys().cloned().collect();
                match &mut self.form {
                    Some(form) => form.show(ui, &existing, &self.views),
                    None => Some(Action::Navigate(Page::Servers)),
                }
            }
            Page::Alerts => pages::alerts::show(ui, state),
            Page::Map => pages::map::show(ui, state, &mut self.map),
            Page::Settings => pages::settings::show(
                ui,
                &SettingsContext {
                    paths: &self.paths,
                    config: &self.config,
                    state,
                },
            ),
        }
    }
}

fn panel_frame(fill: egui::Color32, vertical_margin: i8) -> Frame {
    Frame::new()
        .fill(fill)
        .inner_margin(Margin::symmetric(GAP as i8, vertical_margin))
}

impl eframe::App for AsibaApp {
    fn ui(&mut self, root: &mut egui::Ui, _frame: &mut eframe::Frame) {
        let ctx = root.ctx().clone();
        ctx.request_repaint_after(REPAINT_INTERVAL);
        if let Some(screenshot) = &mut self.screenshot {
            screenshot.tick(&ctx);
        }
        self.apply_start_page(&ctx);
        self.drain_engine_events();
        let palette = theme::Palette::current(&ctx);
        let mut action = None;
        Panel::top("statusbar")
            .exact_size(STATUSBAR_HEIGHT)
            .resizable(false)
            .frame(panel_frame(palette.bg_panel, 0))
            .show(root, |ui| self.statusbar(ui));
        self.sync_pause();
        Panel::left("sidebar")
            .exact_size(SIDEBAR_WIDTH)
            .resizable(false)
            .frame(panel_frame(palette.bg_window, GAP as i8))
            .show(root, |ui| {
                if let Ok(state) = self.state.read() {
                    action = sidebar(ui, &self.page, &state);
                }
            });
        CentralPanel::default_margins()
            .frame(panel_frame(palette.bg_window, GAP as i8))
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
