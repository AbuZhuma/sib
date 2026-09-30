use sib_config::{AI_KEY_ACCOUNT, AiConfig, ThemeChoice};
use sib_core::{
    ActionRequest, ActionSpec, AlertRule, Credentials, Environment, ModuleId, QueryRequest,
    ServerId, ServerSpec,
};
use sib_engine::{AlertSettings, Command};
use sib_modules::files;

use super::SibApp;
use super::confirm::ConfirmDialog;
use crate::modules::Tab;
use crate::pages::Page;
use crate::pages::inspector::Inspector;
use crate::pages::server_form::ServerForm;
use crate::shell::Notice;
use crate::text;
use crate::theme;

impl SibApp {
    pub(super) fn save_config(&mut self) {
        if let Err(error) = self.config.save(&self.paths) {
            self.notices.push(Notice::new(error.to_string()));
        }
    }

    pub(super) fn save_server(
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

    pub(super) fn rename_local_records(&mut self, previous: &ServerId, next: &ServerId) {
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

    pub(super) fn open_form(&mut self, id: Option<ServerId>) {
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

    pub(super) fn sync_ignored(&mut self) {
        if let Err(error) = self.ignored.save(&self.paths) {
            self.notices.push(Notice::new(error.to_string()));
        }
        self.engine
            .send(Command::SetIgnoredIncidents(self.ignored.incidents.clone()));
    }

    pub(super) fn start_query(
        &mut self,
        server: ServerId,
        module: ModuleId,
        request: QueryRequest,
    ) {
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

    pub(super) fn refresh_files(&mut self, server: ServerId, path: String) {
        if let Some(browser) = self.files.get_mut(&server) {
            browser.forget_subtree(&path);
        }
        self.start_files_query(server, QueryRequest::new(files::QUERY_LIST, path));
    }

    pub(super) fn ask_perform(
        &mut self,
        server: ServerId,
        spec: ActionSpec,
        request: ActionRequest,
    ) {
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

    pub(super) fn save_alert_settings(
        &mut self,
        rules: Vec<AlertRule>,
        desktop_notifications: bool,
    ) {
        self.config.alert_rules = rules;
        self.config.desktop_notifications = desktop_notifications;
        self.engine
            .send(Command::SetAlertSettings(AlertSettings::from_custom(
                &self.config.alert_rules,
                desktop_notifications,
            )));
        self.save_config();
    }

    pub(super) fn save_ai_config(&mut self, config: AiConfig) {
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
        self.save_config();
    }

    pub(super) fn set_theme(&mut self, ctx: &egui::Context, choice: ThemeChoice) {
        self.config.theme = choice;
        theme::apply(ctx, choice);
        self.save_config();
    }
}
