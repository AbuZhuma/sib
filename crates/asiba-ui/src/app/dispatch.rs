use asiba_engine::Command;

use super::AsibaApp;
use super::dialogs::DeleteDialog;
use crate::pages::Action;

impl AsibaApp {
    pub(super) fn apply(&mut self, action: Action, ctx: &egui::Context) {
        match action {
            Action::Navigate(_)
            | Action::OpenForm(_)
            | Action::SaveServer { .. }
            | Action::TestConnection(_)
            | Action::Reconnect(_)
            | Action::TrustHostKey { .. }
            | Action::OpenServerFile(_)
            | Action::OpenTerminal(_)
            | Action::AskDelete(_) => self.apply_server(action),
            Action::Query { .. }
            | Action::Backfill { .. }
            | Action::AskPerform { .. }
            | Action::CloseInspector
            | Action::FilesQuery { .. }
            | Action::FilesRefresh { .. }
            | Action::FilesClearSearch { .. } => self.apply_data(action),
            Action::SetTheme(_)
            | Action::SetGeolocation(_)
            | Action::SaveCollection { .. }
            | Action::SaveAlertSettings { .. }
            | Action::SaveServerChecks { .. }
            | Action::SaveAiConfig(_) => self.apply_settings(action, ctx),
            Action::AcknowledgeAlert(_)
            | Action::MuteAlert { .. }
            | Action::Audit { .. }
            | Action::CancelAudit(_)
            | Action::IgnoreIncident(_)
            | Action::RestoreIncident(_) => self.apply_incidents(action),
        }
    }

    fn apply_server(&mut self, action: Action) {
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
            Action::OpenTerminal(id) => self.open_terminal(&id),
            Action::AskDelete(id) => {
                self.delete_dialog = Some(DeleteDialog {
                    id,
                    typed: String::new(),
                })
            }
            _ => {}
        }
    }

    fn apply_data(&mut self, action: Action) {
        match action {
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
            _ => {}
        }
    }

    fn apply_settings(&mut self, action: Action, ctx: &egui::Context) {
        match action {
            Action::SetTheme(choice) => self.set_theme(ctx, choice),
            Action::SetGeolocation(enabled) => {
                self.config.geolocation = enabled;
                self.engine.send(Command::SetGeolocation(enabled));
                self.save_config();
            }
            Action::SaveServerChecks { server, checks } => {
                self.engine
                    .send(Command::SetServerChecks { server, checks });
            }
            Action::SaveCollection {
                intervals,
                retention,
            } => {
                self.config.intervals = intervals;
                self.config.retention = retention;
                self.engine.send(Command::SetIntervals(intervals));
                self.engine.send(Command::SetRetention(retention));
                self.save_config();
            }
            Action::SaveAlertSettings {
                rules,
                desktop_notifications,
            } => self.save_alert_settings(rules, desktop_notifications),
            Action::SaveAiConfig(config) => self.save_ai_config(config),
            _ => {}
        }
    }

    fn apply_incidents(&mut self, action: Action) {
        match action {
            Action::AcknowledgeAlert(id) => self.engine.send(Command::AcknowledgeAlert(id)),
            Action::MuteAlert { id, until } => self.engine.send(Command::MuteAlert { id, until }),
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
            _ => {}
        }
    }
}
