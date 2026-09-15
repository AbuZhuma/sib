use asiba_config::ThemeChoice;
use asiba_core::{
    ActionRequest, ActionSpec, AlertRule, Environment, ModuleId, QueryRequest, ServerId,
};
use asiba_engine::{AlertSettings, Command};

use super::AsibaApp;
use super::confirm::ConfirmDialog;
use super::dialogs::DeleteDialog;
use crate::pages::inspector::Inspector;
use crate::pages::server_form::ServerForm;
use crate::pages::{Action, Page};
use crate::shell::Notice;
use crate::theme;

impl AsibaApp {
    pub(super) fn apply(&mut self, action: Action, ctx: &egui::Context) {
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
            Action::OpenServerFile(id) => self.open_server_file(&id),
            Action::SaveLayout { server, layout } => {
                self.layouts.servers.insert(server.to_string(), layout);
                if let Err(error) = self.layouts.save(&self.paths) {
                    self.notices.push(Notice::new(error.to_string()));
                }
            }
            Action::OpenTerminal(id) => self.open_terminal(&id),
            Action::AskDelete(id) => {
                self.delete_dialog = Some(DeleteDialog {
                    id,
                    typed: String::new(),
                })
            }
            Action::SetTheme(choice) => self.set_theme(ctx, choice),
            Action::Query {
                server,
                module,
                request,
            } => self.start_query(server, module, request),
            Action::AskPerform {
                server,
                spec,
                request,
            } => self.ask_perform(server, spec, request),
            Action::CloseInspector => self.inspector = None,
            Action::AcknowledgeAlert(id) => self.engine.send(Command::AcknowledgeAlert(id)),
            Action::MuteAlert { id, until } => self.engine.send(Command::MuteAlert { id, until }),
            Action::SaveCollection {
                intervals,
                retention,
            } => {
                self.config.intervals = intervals;
                self.config.retention = retention;
                self.engine.send(Command::SetIntervals(intervals));
                if let Err(error) = self.config.save(&self.paths) {
                    self.notices.push(Notice::new(error.to_string()));
                }
            }
            Action::SaveAlertSettings {
                rules,
                desktop_notifications,
            } => self.save_alert_settings(rules, desktop_notifications),
            Action::SaveLlmConfig(config) => {
                self.config.llm = config.clone();
                self.engine.send(Command::SetLlmConfig(config));
                if let Err(error) = self.config.save(&self.paths) {
                    self.notices.push(Notice::new(error.to_string()));
                }
            }
            Action::UnloadModel => self.engine.send(Command::UnloadModel),
            Action::Audit { server, scope } => self.engine.send(Command::Audit {
                server,
                scope,
                is_auto: false,
            }),
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

    fn start_query(&mut self, server: ServerId, module: ModuleId, request: QueryRequest) {
        let token = self.next_query_token;
        self.next_query_token += 1;
        self.inspector = Some(Inspector {
            token,
            server: server.clone(),
            title: format!("{} {}", request.kind, request.target),
            result: None,
        });
        self.engine.send(Command::Query {
            token,
            server,
            module,
            request,
        });
    }

    fn ask_perform(&mut self, server: ServerId, spec: ActionSpec, request: ActionRequest) {
        let environment = self
            .state
            .read()
            .ok()
            .and_then(|state| {
                state
                    .servers
                    .get(&server)
                    .map(|s| s.spec.description.environment)
            })
            .unwrap_or(Environment::Production);
        self.confirm_dialog = Some(ConfirmDialog::new(server, spec, request, environment));
    }

    fn save_alert_settings(&mut self, rules: Vec<AlertRule>, desktop_notifications: bool) {
        self.config.alert_rules = rules;
        self.config.desktop_notifications = desktop_notifications;
        self.engine
            .send(Command::SetAlertSettings(AlertSettings::from_custom(
                &self.config.alert_rules,
                desktop_notifications,
            )));
        if let Err(error) = self.config.save(&self.paths) {
            self.notices.push(Notice::new(error.to_string()));
        }
    }

    fn set_theme(&mut self, ctx: &egui::Context, choice: ThemeChoice) {
        self.config.theme = choice;
        theme::apply(ctx, choice);
        if let Err(error) = self.config.save(&self.paths) {
            self.notices.push(Notice::new(error.to_string()));
        }
    }
}
