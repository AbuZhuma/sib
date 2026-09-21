use asiba_config::{AI_KEY_ACCOUNT, AiConfig, ThemeChoice};
use asiba_core::{
    ActionRequest, ActionSpec, AlertRule, Credentials, Environment, ModuleId, QueryRequest,
    ServerId, ServerSpec,
};
use asiba_engine::{AlertSettings, Command};
use asiba_modules::files;

use super::AsibaApp;
use super::confirm::ConfirmDialog;
use super::dialogs::DeleteDialog;
use crate::modules::Tab;
use crate::pages::inspector::Inspector;
use crate::pages::server_form::ServerForm;
use crate::pages::{Action, Page};
use crate::shell::Notice;
use crate::text;
use crate::theme;

impl AsibaApp {
    pub(super) fn apply(&mut self, action: Action, ctx: &egui::Context) {
        match action {
            Action::Navigate(page) => self.page = page,
            Action::OpenForm(id) => self.open_form(id),
            Action::SaveServer {
                spec,
                credentials,
                previous,
            } => self.save_server(spec, credentials, previous),
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
            Action::SetGeolocation(enabled) => {
                self.config.geolocation = enabled;
                self.engine.send(Command::SetGeolocation(enabled));
                if let Err(error) = self.config.save(&self.paths) {
                    self.notices.push(Notice::new(error.to_string()));
                }
            }
            Action::Query {
                server,
                module,
                request,
            } => self.start_query(server, module, request),
            Action::Backfill { server, module } => {
                self.engine.send(Command::Backfill { server, module });
            }
            Action::AskPerform {
                server,
                spec,
                request,
            } => self.ask_perform(server, spec, request),
            Action::CloseInspector => self.inspector = None,
            Action::FilesQuery { server, request } => self.start_files_query(server, request),
            Action::FilesRefresh { server, path } => self.refresh_files(server, path),
            Action::FilesClearSearch { server } => {
                if let Some(browser) = self.files.get_mut(&server) {
                    browser.clear_search();
                }
            }
            Action::AcknowledgeAlert(id) => self.engine.send(Command::AcknowledgeAlert(id)),
            Action::MuteAlert { id, until } => self.engine.send(Command::MuteAlert { id, until }),
            Action::SaveCollection {
                intervals,
                retention,
            } => {
                self.config.intervals = intervals;
                self.config.retention = retention;
                self.engine.send(Command::SetIntervals(intervals));
                self.engine.send(Command::SetRetention(retention));
                if let Err(error) = self.config.save(&self.paths) {
                    self.notices.push(Notice::new(error.to_string()));
                }
            }
            Action::SaveAlertSettings {
                rules,
                desktop_notifications,
            } => self.save_alert_settings(rules, desktop_notifications),
            Action::SaveAiConfig(config) => self.save_ai_config(config),
            Action::Audit { target, scope } => self.engine.send(Command::Audit {
                target,
                scope,
                is_auto: false,
            }),
            Action::CancelAudit(id) => self.engine.send(Command::CancelAudit(id)),
            Action::IgnoreIncident(entry) => {
                self.ignored.add(entry);
                self.sync_ignored();
            }
            Action::RestoreIncident(entry) => {
                self.ignored.remove(&entry);
                self.sync_ignored();
            }
        }
    }

    fn save_server(
        &mut self,
        spec: ServerSpec,
        credentials: Credentials,
        previous: Option<ServerId>,
    ) {
        let id = spec.id.clone();
        let command = match previous {
            None => Command::AddServer { spec, credentials },
            Some(previous) => {
                if previous != id {
                    self.rename_local_records(&previous, &id);
                }
                Command::UpdateServer {
                    previous,
                    spec,
                    credentials,
                }
            }
        };
        self.engine.send(command);
        self.form = None;
        self.page = Page::ServerDetail(id);
    }

    fn rename_local_records(&mut self, previous: &ServerId, next: &ServerId) {
        if let Some(layout) = self.layouts.servers.remove(previous.as_str()) {
            self.layouts.servers.insert(next.to_string(), layout);
            if let Err(error) = self.layouts.save(&self.paths) {
                self.notices.push(Notice::new(error.to_string()));
            }
        }
        let has_ignored = self.ignored.incidents.iter().any(|i| &i.server == previous);
        if has_ignored {
            for entry in &mut self.ignored.incidents {
                if &entry.server == previous {
                    entry.server = next.clone();
                }
            }
            self.sync_ignored();
        }
        self.files.remove(previous);
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

    fn sync_ignored(&mut self) {
        if let Err(error) = self.ignored.save(&self.paths) {
            self.notices.push(Notice::new(error.to_string()));
        }
        self.engine
            .send(Command::SetIgnoredIncidents(self.ignored.incidents.clone()));
    }

    fn start_query(&mut self, server: ServerId, module: ModuleId, request: QueryRequest) {
        let token = self.next_query_token;
        self.next_query_token += 1;
        self.inspector = Some(Inspector {
            token,
            server: server.clone(),
            tab: Tab::for_module(module.0),
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

    pub(super) fn start_files_query(&mut self, server: ServerId, request: QueryRequest) {
        let token = self.next_query_token;
        self.next_query_token += 1;
        self.files
            .entry(server.clone())
            .or_default()
            .start(token, &request);
        self.engine.send(Command::Query {
            token,
            server,
            module: files::ID,
            request,
        });
    }

    fn refresh_files(&mut self, server: ServerId, path: String) {
        if let Some(browser) = self.files.get_mut(&server) {
            browser.forget_subtree(&path);
        }
        self.start_files_query(server, QueryRequest::new(files::QUERY_LIST, path));
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

    fn save_ai_config(&mut self, config: AiConfig) {
        let stored = self
            .secrets
            .set_or_delete_named(AI_KEY_ACCOUNT, &config.api_key);
        if let Err(error) = stored {
            self.notices.push(Notice::new(format!(
                "{}: {error}",
                text::ERR_AI_KEY_NOT_STORED
            )));
        }
        self.config.ai = config.clone();
        self.engine.send(Command::SetAiConfig(config));
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
