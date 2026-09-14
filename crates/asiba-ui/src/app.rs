use std::sync::Arc;
use std::time::Duration;

use asiba_config::{AppConfig, Paths, ThemeChoice};
use asiba_core::{ServerId, SharedState};
use asiba_engine::{Command, EngineEvent, EngineHandle, RepaintNotifier};
use egui::{CentralPanel, Frame, Margin, Panel};

use crate::devtools::ScreenshotOnStart;
use crate::modules::{self, ModuleView};
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
            Ok(Box::new(AsibaApp::new(deps, engine)))
        }),
    )
}

struct DeleteDialog {
    id: ServerId,
    typed: String,
}

struct AsibaApp {
    engine: EngineHandle,
    state: SharedState,
    paths: Paths,
    config: AppConfig,
    page: Page,
    form: Option<ServerForm>,
    delete_dialog: Option<DeleteDialog>,
    notices: Vec<Notice>,
    views: Vec<Box<dyn ModuleView>>,
    screenshot: Option<ScreenshotOnStart>,
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
            notices: Vec::new(),
            views: modules::all(),
            screenshot: ScreenshotOnStart::from_env(),
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
                EngineEvent::ServerSaved(_) | EngineEvent::ServerRemoved(_) => {}
                EngineEvent::Warning(message) => self.notices.push(Notice::new(message)),
            }
        }
        self.notices.retain(|n| !n.is_expired());
    }

    fn apply(&mut self, action: Action, ctx: &egui::Context) {
        match action {
            Action::Navigate(page) => self.page = page,
            Action::OpenForm(id) => self.open_form(id),
            Action::SaveServer {
                spec,
                credentials,
                is_new,
            } => {
                let id = spec.id.clone();
                let command = if is_new {
                    Command::AddServer { spec, credentials }
                } else {
                    Command::UpdateServer { spec, credentials }
                };
                self.engine.send(command);
                self.form = None;
                self.page = Page::ServerDetail(id);
            }
            Action::TestConnection(request) => self.engine.send(Command::TestConnection(request)),
            Action::Reconnect(id) => self.engine.send(Command::Reconnect(id)),
            Action::TrustHostKey {
                server,
                fingerprint,
            } => {
                self.engine.send(Command::TrustHostKey {
                    server,
                    fingerprint,
                });
            }
            Action::AskDelete(id) => {
                self.delete_dialog = Some(DeleteDialog {
                    id,
                    typed: String::new(),
                })
            }
            Action::SetTheme(choice) => self.set_theme(ctx, choice),
        }
    }

    fn open_form(&mut self, id: Option<ServerId>) {
        let spec = id.and_then(|id| {
            self.state
                .read()
                .ok()
                .and_then(|state| state.servers.get(&id).map(|s| s.spec.clone()))
        });
        self.form = Some(match spec {
            Some(spec) => ServerForm::edit(&spec),
            None => ServerForm::new(),
        });
        self.page = Page::ServerForm;
    }

    fn set_theme(&mut self, ctx: &egui::Context, choice: ThemeChoice) {
        self.config.theme = choice;
        theme::apply(ctx, choice);
        if let Err(error) = self.config.save(&self.paths) {
            self.notices.push(Notice::new(error.to_string()));
        }
    }

    fn central(&mut self, ui: &mut egui::Ui) -> Option<Action> {
        let Ok(state) = self.state.read() else {
            return None;
        };
        match self.page.clone() {
            Page::Overview => pages::overview::show(ui, &state),
            Page::Servers => pages::servers::show(ui, &state),
            Page::ServerDetail(id) => match state.servers.get(&id) {
                Some(server) => pages::server_detail::show(ui, server, &self.views),
                None => Some(Action::Navigate(Page::Servers)),
            },
            Page::ServerForm => {
                let existing: Vec<ServerId> = state.servers.keys().cloned().collect();
                match &mut self.form {
                    Some(form) => form.show(ui, &existing, &self.views),
                    None => Some(Action::Navigate(Page::Servers)),
                }
            }
            Page::Alerts => {
                pages::alerts::show(ui);
                None
            }
            Page::Map => {
                pages::map::show(ui);
                None
            }
            Page::Settings => pages::settings::show(ui, &self.paths, &self.config),
        }
    }

    fn delete_modal(&mut self, ctx: &egui::Context) {
        let Some(dialog) = &mut self.delete_dialog else {
            return;
        };
        let mut close = false;
        let mut confirmed = None;
        egui::Modal::new(egui::Id::new("delete-server")).show(ctx, |ui| {
            ui.set_width(360.0);
            ui.heading(format!("{} {}", text::BTN_DELETE, dialog.id));
            ui.label(text::DETAIL_DELETE_PROMPT);
            ui.text_edit_singleline(&mut dialog.typed);
            ui.horizontal(|ui| {
                let matches = dialog.typed.trim() == dialog.id.as_str();
                if ui
                    .add_enabled(matches, egui::Button::new(text::BTN_CONFIRM_DELETE))
                    .clicked()
                {
                    confirmed = Some(dialog.id.clone());
                }
                if ui.button(text::BTN_CANCEL).clicked() {
                    close = true;
                }
            });
        });
        if let Some(id) = confirmed {
            self.engine.send(Command::RemoveServer(id));
            self.page = Page::Servers;
            close = true;
        }
        if close {
            self.delete_dialog = None;
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
        if let Some(action) = action {
            self.apply(action, &ctx);
        }
    }
}
