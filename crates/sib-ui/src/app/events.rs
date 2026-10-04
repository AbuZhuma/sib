use sib_core::{ActionRecord, AuditStatus, QueryRequest, QueryResponse};
use sib_engine::EngineEvent;
use sib_modules::files;

use super::SibApp;
use crate::shell::Notice;
use crate::text;

const MAX_INCIDENT_NOTICES: usize = 3;

impl SibApp {
    pub(super) fn drain_engine_events(&mut self) {
        for event in self.engine.poll_events() {
            match event {
                EngineEvent::TestFinished(report) => {
                    if let Some(form) = &mut self.form {
                        form.accept_report(report);
                    }
                }
                EngineEvent::QueryFinished { token, result } => {
                    self.accept_query(token, result);
                }
                EngineEvent::ActionFinished(record) => {
                    self.refresh_after_file_action(&record);
                    if let Some(message) = notice_message(EngineEvent::ActionFinished(record)) {
                        self.notices.push(Notice::new(message));
                    }
                }
                EngineEvent::IncidentsOpened(incidents) => {
                    for incident in incidents.iter().take(MAX_INCIDENT_NOTICES) {
                        self.notices.push(Notice::new(format!(
                            "{} {}: {}",
                            text::INCIDENT_NOTICE,
                            incident.server,
                            incident.summary
                        )));
                    }
                }
                other => {
                    if let Some(message) = notice_message(other) {
                        self.notices.push(Notice::new(message));
                    }
                }
            }
        }
        self.notices.retain(|n| !n.is_expired());
    }
}

impl SibApp {
    fn accept_query(&mut self, token: u64, result: Result<QueryResponse, String>) {
        if let Some(browser) = self.files.values_mut().find(|b| b.has_pending(token)) {
            browser.accept(token, result.map(|r| r.text));
            return;
        }
        if let Some(inspector) = &mut self.inspector {
            inspector.accept(token, result.map(|r| (r.title, r.text)));
        }
    }

    fn refresh_after_file_action(&mut self, record: &ActionRecord) {
        if record.module != files::ID.0 || !record.is_success {
            return;
        }
        let server = record.server.clone();
        if record.kind == files::ACTION_WRITE
            && let Some(browser) = self.files.get_mut(&server)
        {
            browser.forget_content(&record.target);
            self.start_files_query(
                server.clone(),
                QueryRequest::new(files::QUERY_READ, &record.target),
            );
        }
        if matches!(
            record.kind.as_str(),
            files::ACTION_MOVE | files::ACTION_COPY
        ) && let Some(destination) = &record.argument
        {
            let parent = files::parent_path(destination).to_owned();
            self.start_files_query(server.clone(), QueryRequest::new(files::QUERY_LIST, parent));
        }
        if let Some(browser) = self.files.get_mut(&server) {
            browser.forget_subtree(&record.target);
        }
        let parent = files::parent_path(&record.target).to_owned();
        self.start_files_query(server, QueryRequest::new(files::QUERY_LIST, parent));
    }
}

fn notice_message(event: EngineEvent) -> Option<String> {
    match event {
        EngineEvent::ActionFinished(record) => {
            let outcome = if record.is_success {
                text::ACTION_DONE
            } else {
                text::ACTION_FAILED
            };
            Some(format!(
                "{} {}: {outcome} - {}",
                record.kind, record.target, record.message
            ))
        }
        EngineEvent::AuditFinished(report) => Some(match &report.status {
            AuditStatus::Failed(error) => {
                format!(
                    "{} {}: {error}",
                    text::AUDIT_NOTICE_FAILED,
                    report.target.key()
                )
            }
            _ => format!(
                "{} {}: {}",
                text::AUDIT_NOTICE_DONE,
                report.target.key(),
                report.scope.key()
            ),
        }),
        EngineEvent::PipelineFinished {
            server,
            name,
            is_success,
            ..
        } => Some(format!(
            "{}: {name} @ {server}",
            if is_success {
                text::RUN_NOTICE_DONE
            } else {
                text::RUN_NOTICE_FAILED
            }
        )),
        EngineEvent::Warning(message) => Some(message),
        EngineEvent::TestFinished(_)
        | EngineEvent::QueryFinished { .. }
        | EngineEvent::IncidentsOpened(_)
        | EngineEvent::ServerSaved(_)
        | EngineEvent::ServerRemoved(_) => None,
    }
}
