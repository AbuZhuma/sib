use asiba_config::ThemeChoice;
use asiba_core::{ModuleId, QueryRequest, ServerId};
use asiba_engine::Command;

use super::AsibaApp;
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
            Action::CloseInspector => self.inspector = None,
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

    fn set_theme(&mut self, ctx: &egui::Context, choice: ThemeChoice) {
        self.config.theme = choice;
        theme::apply(ctx, choice);
        if let Err(error) = self.config.save(&self.paths) {
            self.notices.push(Notice::new(error.to_string()));
        }
    }
}
