use asiba_llm::InstallTarget;
use tokio::sync::mpsc;

use crate::command::EngineEvent;

pub fn install(target: InstallTarget, events: mpsc::UnboundedSender<EngineEvent>) {
    tokio::task::spawn_blocking(move || {
        let progress_events = events.clone();
        let report = move |progress| {
            let _ = progress_events.send(EngineEvent::LlmInstallProgress(progress));
        };
        tracing::info!(dir = %target.llama_dir.display(), "установка llama.cpp и модели");
        let result = asiba_llm::install(&target, &report).map_err(|e| e.to_string());
        match &result {
            Ok(installed) => {
                tracing::info!(model = %installed.model_path.display(), "модель установлена")
            }
            Err(error) => tracing::warn!("установка модели не удалась: {error}"),
        }
        let _ = events.send(EngineEvent::LlmInstalled(result));
    });
}
