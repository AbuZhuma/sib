use std::collections::BTreeMap;
use std::path::Path;

use sib_config::PipelineStore;
use sib_core::{Environment, Pipeline, PipelineBinding, ServerId};
use sib_engine::Command;

use super::SibApp;
use super::run_dialog::RunDialog;
use crate::pages::Action;
use crate::shell::Notice;
use crate::text;

impl SibApp {
    pub(super) fn apply_pipelines(&mut self, action: Action) {
        match action {
            Action::SavePipeline(pipeline) => self.save_pipeline(pipeline),
            Action::DeletePipeline(id) => self.delete_pipeline(&id),
            Action::ImportPipeline(path) => self.import_pipeline(&path),
            Action::ExportPipeline { id, path } => self.export_pipeline(&id, &path),
            Action::SaveServerPipelines { server, bindings } => {
                self.save_server_pipelines(server, bindings);
            }
            Action::AskRunPipeline {
                server,
                pipeline_id,
            } => self.ask_run_pipeline(server, &pipeline_id),
            Action::CancelPipeline(run_id) => self.engine.send(Command::CancelPipeline(run_id)),
            _ => {}
        }
    }

    fn reload_prototypes(&mut self) {
        self.prototypes = self.pipelines.load_all();
    }

    fn save_pipeline(&mut self, pipeline: Pipeline) {
        if let Err(error) = pipeline.validate() {
            self.notices.push(Notice::new(error.to_string()));
            return;
        }
        match self.pipelines.save(&pipeline) {
            Ok(()) => self.notices.push(Notice::new(format!(
                "{}: {}",
                text::PIPELINE_SAVED,
                pipeline.name
            ))),
            Err(error) => self.notices.push(Notice::new(error.to_string())),
        }
        self.reload_prototypes();
    }

    fn delete_pipeline(&mut self, id: &str) {
        match self.pipelines.delete(id) {
            Ok(()) => self
                .notices
                .push(Notice::new(format!("{}: {id}", text::PIPELINE_DELETED))),
            Err(error) => self.notices.push(Notice::new(error.to_string())),
        }
        self.reload_prototypes();
    }

    fn import_pipeline(&mut self, path: &Path) {
        match PipelineStore::read(path)
            .map_err(|e| e.to_string())
            .and_then(|pipeline| {
                pipeline
                    .validate()
                    .map(|()| pipeline)
                    .map_err(|e| e.to_string())
            }) {
            Ok(pipeline) => {
                let name = pipeline.name.clone();
                if let Err(error) = self.pipelines.save(&pipeline) {
                    self.notices.push(Notice::new(error.to_string()));
                } else {
                    self.notices
                        .push(Notice::new(format!("{}: {name}", text::IMPORTED)));
                }
                self.reload_prototypes();
            }
            Err(error) => self.notices.push(Notice::new(error)),
        }
    }

    fn export_pipeline(&mut self, id: &str, path: &Path) {
        let Some(pipeline) = self.prototypes.iter().find(|p| p.id == id) else {
            self.notices
                .push(Notice::new(text::ERR_PIPELINE_NOT_FOUND.to_owned()));
            return;
        };
        match self.pipelines.export(pipeline, path) {
            Ok(()) => self.notices.push(Notice::new(format!(
                "{} {}",
                text::EXPORTED_TO,
                path.display()
            ))),
            Err(error) => self.notices.push(Notice::new(error.to_string())),
        }
    }

    fn save_server_pipelines(&mut self, server: ServerId, bindings: Vec<PipelineBinding>) {
        self.engine
            .send(Command::SetServerPipelines { server, bindings });
    }

    fn ask_run_pipeline(&mut self, server: ServerId, pipeline_id: &str) {
        let Some(pipeline) = self
            .prototypes
            .iter()
            .find(|p| p.id == pipeline_id)
            .cloned()
        else {
            self.notices
                .push(Notice::new(text::ERR_PIPELINE_NOT_FOUND.to_owned()));
            return;
        };
        let (bound, environment, is_online) = self
            .state
            .read()
            .ok()
            .and_then(|state| {
                let entry = state.servers.get(&server)?;
                let bound = entry
                    .spec
                    .pipelines
                    .iter()
                    .find(|b| b.pipeline == pipeline_id)
                    .map(|b| b.values.clone())
                    .unwrap_or_default();
                Some((
                    bound,
                    entry.spec.description.environment,
                    entry.connection.is_online(),
                ))
            })
            .unwrap_or((BTreeMap::new(), Environment::Production, false));
        self.run_dialog = Some(RunDialog::new(
            server,
            pipeline,
            &bound,
            environment,
            is_online,
        ));
    }
}
