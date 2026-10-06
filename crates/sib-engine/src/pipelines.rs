use std::collections::BTreeMap;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

use chrono::Utc;
use sib_core::pipeline::{builtin_values, render};
use sib_core::{
    ActionRecord, OutputChunk, Pipeline, PipelineRun, RunStatus, ServerSpec, SharedState, StepRun,
    StepStatus, StepTarget, SudoMode, Transport,
};
use sib_storage::{HistoryReader, StorageWriter};
use sib_transport::LocalTransport;
use tokio::sync::mpsc;

use crate::ai::Cancellations;
use crate::command::EngineEvent;
use crate::engine::RepaintNotifier;

const RUNS_PREFILL: usize = 100;
const JOURNAL_MODULE: &str = "pipeline";
const JOURNAL_KIND: &str = "run";

pub struct RunJob {
    pub run_id: u64,
    pub spec: ServerSpec,
    pub pipeline: Pipeline,
    pub values: BTreeMap<String, String>,
    pub transport: Option<Arc<dyn Transport>>,
    pub state: SharedState,
    pub storage: Option<StorageWriter>,
    pub events: mpsc::UnboundedSender<EngineEvent>,
    pub notify: RepaintNotifier,
    pub cancellations: Cancellations,
}

pub fn effective_values(
    spec: &ServerSpec,
    pipeline: &Pipeline,
    provided: &BTreeMap<String, String>,
) -> BTreeMap<String, String> {
    let mut values = builtin_values(spec);
    for variable in &pipeline.variables {
        values.insert(variable.name.clone(), variable.default.clone());
    }
    for (name, value) in provided {
        if !value.trim().is_empty() {
            values.insert(name.clone(), value.clone());
        }
    }
    values
}

pub fn start(job: RunJob) {
    let run = PipelineRun {
        id: job.run_id,
        server: job.spec.id.clone(),
        pipeline: job.pipeline.id.clone(),
        pipeline_name: job.pipeline.name.clone(),
        started_at: Utc::now(),
        finished_at: None,
        status: RunStatus::Running,
        steps: job.pipeline.steps.iter().map(StepRun::pending).collect(),
    };
    if let Ok(mut state) = job.state.write() {
        state.push_pipeline_run(run);
    }
    (job.notify)();
    tokio::spawn(async move {
        let status = execute(&job).await;
        finish(&job, status);
    });
}

async fn execute(job: &RunJob) -> RunStatus {
    let values = effective_values(&job.spec, &job.pipeline, &job.values);
    let local: Arc<dyn Transport> = Arc::new(LocalTransport::new(SudoMode::None, None));
    for (index, step) in job.pipeline.steps.iter().enumerate() {
        if job.cancellations.is_cancelled(job.run_id) {
            skip_rest(job, index);
            roll_back(job, &values, &local, index).await;
            return RunStatus::Cancelled;
        }
        let command = match render(&step.command, &values) {
            Ok(command) => command,
            Err(error) => {
                mark(job, index, StepStatus::Failed, None);
                append(job, index, &format!("{error}\n"));
                skip_rest(job, index + 1);
                roll_back(job, &values, &local, index).await;
                return RunStatus::Failed(format!("step '{}': {error}", step.name));
            }
        };
        let transport = match transport_for(job, &local, step.target) {
            Some(transport) => transport,
            None => {
                mark(job, index, StepStatus::Failed, None);
                skip_rest(job, index + 1);
                roll_back(job, &values, &local, index).await;
                return RunStatus::Failed("server is not connected".to_owned());
            }
        };
        mark(job, index, StepStatus::Running, None);
        let timeout = (step.timeout_secs > 0).then(|| Duration::from_secs(step.timeout_secs));
        let sink = |chunk: OutputChunk| append(job, index, &chunk.text);
        let result = tokio::select! {
            result = transport.exec_streaming(&command, step.as_root, timeout, &sink) => Some(result),
            () = job.cancellations.wait_for(job.run_id) => None,
        };
        match result {
            None => {
                mark(job, index, StepStatus::Failed, None);
                append(job, index, "\ncancelled\n");
                skip_rest(job, index + 1);
                roll_back(job, &values, &local, index).await;
                return RunStatus::Cancelled;
            }
            Some(Ok(0)) => mark(job, index, StepStatus::Done, Some(0)),
            Some(Ok(code)) => {
                mark(job, index, StepStatus::Failed, Some(code));
                if !step.continue_on_error {
                    skip_rest(job, index + 1);
                    roll_back(job, &values, &local, index).await;
                    return RunStatus::Failed(format!("step '{}' exited with {code}", step.name));
                }
            }
            Some(Err(error)) => {
                mark(job, index, StepStatus::Failed, None);
                append(job, index, &format!("\n{error}\n"));
                if !step.continue_on_error {
                    skip_rest(job, index + 1);
                    roll_back(job, &values, &local, index).await;
                    return RunStatus::Failed(format!("step '{}': {error}", step.name));
                }
            }
        }
    }
    RunStatus::Done
}

fn transport_for(
    job: &RunJob,
    local: &Arc<dyn Transport>,
    target: StepTarget,
) -> Option<Arc<dyn Transport>> {
    match target {
        StepTarget::Local => Some(Arc::clone(local)),
        StepTarget::Server => job.transport.as_ref().map(Arc::clone),
    }
}

fn status_of(job: &RunJob, index: usize) -> Option<StepStatus> {
    job.state.read().ok().and_then(|s| {
        s.pipeline_runs
            .iter()
            .find(|r| r.id == job.run_id)
            .map(|r| r.steps[index].status)
    })
}

async fn roll_back(
    job: &RunJob,
    values: &BTreeMap<String, String>,
    local: &Arc<dyn Transport>,
    failed_index: usize,
) {
    for index in (0..failed_index).rev() {
        let step = &job.pipeline.steps[index];
        let Some(rollback) = step.rollback_command() else {
            continue;
        };
        if status_of(job, index) != Some(StepStatus::Done) {
            continue;
        }
        append(job, index, "\n--- rollback\n");
        let command = match render(rollback, values) {
            Ok(command) => command,
            Err(error) => {
                append(job, index, &format!("rollback not rendered: {error}\n"));
                continue;
            }
        };
        let Some(transport) = transport_for(job, local, step.target) else {
            append(job, index, "rollback skipped: server is not connected\n");
            continue;
        };
        let timeout = (step.timeout_secs > 0).then(|| Duration::from_secs(step.timeout_secs));
        let sink = |chunk: OutputChunk| append(job, index, &chunk.text);
        match transport
            .exec_streaming(&command, step.as_root, timeout, &sink)
            .await
        {
            Ok(0) => mark(job, index, StepStatus::RolledBack, Some(0)),
            Ok(code) => append(job, index, &format!("rollback exited with {code}\n")),
            Err(error) => append(job, index, &format!("rollback failed: {error}\n")),
        }
    }
}

fn mark(job: &RunJob, index: usize, status: StepStatus, exit_code: Option<i32>) {
    if let Ok(mut state) = job.state.write()
        && let Some(run) = state.pipeline_run_mut(job.run_id)
        && let Some(step) = run.steps.get_mut(index)
    {
        step.status = status;
        step.exit_code = exit_code;
        let now = Utc::now();
        match status {
            StepStatus::Running => step.started_at = Some(now),
            StepStatus::Pending => {}
            _ => step.finished_at = Some(now),
        }
    }
    (job.notify)();
}

fn append(job: &RunJob, index: usize, text: &str) {
    if let Ok(mut state) = job.state.write()
        && let Some(run) = state.pipeline_run_mut(job.run_id)
        && let Some(step) = run.steps.get_mut(index)
    {
        step.append_output(text);
    }
    (job.notify)();
}

fn skip_rest(job: &RunJob, from: usize) {
    if let Ok(mut state) = job.state.write()
        && let Some(run) = state.pipeline_run_mut(job.run_id)
    {
        for step in run.steps.iter_mut().skip(from) {
            if step.status == StepStatus::Pending {
                step.status = StepStatus::Skipped;
            }
        }
    }
}

fn finish(job: &RunJob, status: RunStatus) {
    job.cancellations.take(job.run_id);
    let finished = job.state.write().ok().and_then(|mut state| {
        let run = state.pipeline_run_mut(job.run_id)?;
        run.status = status.clone();
        run.finished_at = Some(Utc::now());
        Some(run.clone())
    });
    let Some(run) = finished else {
        return;
    };
    let is_success = run.is_success();
    let record = ActionRecord {
        at: Utc::now(),
        server: run.server.clone(),
        module: JOURNAL_MODULE.to_owned(),
        kind: JOURNAL_KIND.to_owned(),
        target: run.pipeline_name.clone(),
        argument: None,
        is_success,
        message: run.summary(),
    };
    if is_success {
        tracing::info!(server = %run.server, pipeline = %run.pipeline, "pipeline done");
    } else {
        tracing::warn!(server = %run.server, pipeline = %run.pipeline, status = ?run.status, "pipeline did not finish");
    }
    if let Some(storage) = &job.storage {
        let _ = storage.write_pipeline_run(run.clone());
        let _ = storage.write_action(record.clone());
    }
    if let Ok(mut state) = job.state.write() {
        state.push_action(record);
    }
    let _ = job.events.send(EngineEvent::PipelineFinished {
        run_id: run.id,
        server: run.server,
        name: run.pipeline_name,
        is_success,
    });
    (job.notify)();
}

pub fn prefill_runs(path: Option<PathBuf>, state: SharedState, notify: RepaintNotifier) {
    let Some(path) = path else {
        return;
    };
    tokio::task::spawn_blocking(move || {
        let loaded =
            HistoryReader::open(&path).and_then(|reader| reader.recent_pipeline_runs(RUNS_PREFILL));
        match loaded {
            Ok(runs) => {
                if let Ok(mut state) = state.write() {
                    let running: Vec<PipelineRun> = state
                        .pipeline_runs
                        .drain(..)
                        .filter(|r| r.is_running())
                        .collect();
                    state.pipeline_runs = runs;
                    state.pipeline_runs.extend(running);
                }
                notify();
            }
            Err(error) => tracing::warn!(%error, "pipeline history was not loaded"),
        }
    });
}

#[cfg(test)]
mod tests {
    use sib_core::{AppState, AuthMethod, ServerDescription, ServerId, Step, Variable};

    use super::*;

    fn spec() -> ServerSpec {
        ServerSpec {
            id: ServerId::parse("local").expect("id"),
            host: "localhost".into(),
            port: 22,
            user: "me".into(),
            auth: AuthMethod::Auto,
            jump: None,
            sudo: SudoMode::None,
            description: ServerDescription::default(),
            location: None,
            modules: Default::default(),
            checks: Vec::new(),
            check_overrides: Default::default(),
            pipelines: Vec::new(),
        }
    }

    fn pipeline(steps: Vec<Step>) -> Pipeline {
        Pipeline {
            id: "t".into(),
            name: "T".into(),
            description: String::new(),
            variables: vec![Variable {
                name: "greeting".into(),
                default: "hi".into(),
                secret: false,
                description: String::new(),
            }],
            steps,
        }
    }

    fn job(pipeline: Pipeline, values: BTreeMap<String, String>) -> (RunJob, SharedState) {
        let state = AppState::shared();
        let (events, _rx) = mpsc::unbounded_channel();
        let job = RunJob {
            run_id: 1,
            spec: spec(),
            pipeline,
            values,
            transport: Some(Arc::new(LocalTransport::new(SudoMode::None, None))),
            state: Arc::clone(&state),
            storage: None,
            events,
            notify: Arc::new(|| {}),
            cancellations: Cancellations::default(),
        };
        (job, state)
    }

    fn local(name: &str, command: &str) -> Step {
        Step {
            name: name.into(),
            command: command.into(),
            target: StepTarget::Local,
            ..Default::default()
        }
    }

    #[test]
    fn effective_values_layer_builtins_defaults_and_provided() {
        let mut provided = BTreeMap::new();
        provided.insert("greeting".to_owned(), "hello".to_owned());
        let values = effective_values(&spec(), &pipeline(Vec::new()), &provided);
        assert_eq!(values.get("greeting").map(String::as_str), Some("hello"));
        assert_eq!(values.get("host").map(String::as_str), Some("localhost"));
        assert_eq!(values.get("ssh_args").map(String::as_str), Some("-p 22"));
    }

    #[test]
    fn empty_provided_value_keeps_the_default() {
        let mut provided = BTreeMap::new();
        provided.insert("greeting".to_owned(), String::new());
        provided.insert("other".to_owned(), "  ".to_owned());
        let pipeline = Pipeline {
            variables: vec![Variable {
                name: "greeting".to_owned(),
                default: "hi".to_owned(),
                secret: false,
                description: String::new(),
            }],
            ..pipeline(Vec::new())
        };
        let values = effective_values(&spec(), &pipeline, &provided);
        assert_eq!(values.get("greeting").map(String::as_str), Some("hi"));
        assert_eq!(values.get("other"), None);
    }

    #[tokio::test]
    async fn run_executes_steps_in_order_and_stops_on_failure() {
        let steps = vec![
            local("one", "echo {{greeting}} {{server}}"),
            local("two", "echo oops >&2; exit 3"),
            local("three", "echo never"),
        ];
        let (job, state) = job(pipeline(steps), BTreeMap::new());
        start(job);
        wait_finished(&state, 1).await;
        let run = state
            .read()
            .ok()
            .and_then(|s| s.pipeline_runs.first().cloned());
        let run = run.expect("run");
        assert_eq!(
            run.status,
            RunStatus::Failed("step 'two' exited with 3".into())
        );
        assert_eq!(run.steps[0].output, "hi local\n");
        assert_eq!(run.steps[0].status, StepStatus::Done);
        assert_eq!(run.steps[1].exit_code, Some(3));
        assert_eq!(run.steps[2].status, StepStatus::Skipped);
        let journal = state.read().ok().map(|s| s.actions.len());
        assert_eq!(journal, Some(1));
    }

    #[tokio::test]
    async fn failure_rolls_back_completed_steps_in_reverse_order() {
        let dir = std::env::temp_dir().join(format!("sib-rollback-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let log = dir.join("log");
        let mut one = local("one", &format!("echo one >> {}", log.display()));
        one.rollback = Some(format!("echo undo-one >> {}", log.display()));
        let mut two = local("two", &format!("echo two >> {}", log.display()));
        two.rollback = Some(format!("echo undo-two >> {}", log.display()));
        let three = local("three", "exit 7");
        let mut four = local("four", "echo never");
        four.rollback = Some("echo never-undone".into());
        let (job, state) = job(pipeline(vec![one, two, three, four]), BTreeMap::new());
        start(job);
        wait_finished(&state, 1).await;
        let run = state
            .read()
            .ok()
            .and_then(|s| s.pipeline_runs.first().cloned())
            .expect("run");
        assert_eq!(
            run.status,
            RunStatus::Failed("step 'three' exited with 7".into())
        );
        assert_eq!(run.steps[0].status, StepStatus::RolledBack);
        assert_eq!(run.steps[1].status, StepStatus::RolledBack);
        assert_eq!(run.steps[2].status, StepStatus::Failed);
        assert_eq!(run.steps[3].status, StepStatus::Skipped);
        assert!(run.steps[0].output.contains("--- rollback"));
        let logged = std::fs::read_to_string(&log).unwrap();
        assert_eq!(logged, "one\ntwo\nundo-two\nundo-one\n");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[tokio::test]
    async fn steps_without_rollback_keep_their_status_on_failure() {
        let (job, state) = job(
            pipeline(vec![local("one", "true"), local("two", "exit 1")]),
            BTreeMap::new(),
        );
        start(job);
        wait_finished(&state, 1).await;
        let run = state
            .read()
            .ok()
            .and_then(|s| s.pipeline_runs.first().cloned())
            .expect("run");
        assert_eq!(run.steps[0].status, StepStatus::Done);
        assert!(!run.steps[0].output.contains("rollback"));
    }

    #[tokio::test]
    async fn continue_on_error_runs_the_next_step() {
        let mut failing = local("one", "exit 1");
        failing.continue_on_error = true;
        let (job, state) = job(
            pipeline(vec![failing, local("two", "echo ok")]),
            BTreeMap::new(),
        );
        start(job);
        wait_finished(&state, 1).await;
        let run = state
            .read()
            .ok()
            .and_then(|s| s.pipeline_runs.first().cloned());
        let run = run.expect("run");
        assert_eq!(run.status, RunStatus::Done);
        assert_eq!(run.steps[1].output, "ok\n");
    }

    #[tokio::test]
    async fn cancel_stops_a_running_step() {
        let (job, state) = job(
            pipeline(vec![local("sleep", "sleep 30"), local("after", "echo x")]),
            BTreeMap::new(),
        );
        let cancellations = job.cancellations.clone();
        start(job);
        tokio::time::sleep(Duration::from_millis(200)).await;
        cancellations.cancel(1);
        wait_finished(&state, 1).await;
        let run = state
            .read()
            .ok()
            .and_then(|s| s.pipeline_runs.first().cloned());
        let run = run.expect("run");
        assert_eq!(run.status, RunStatus::Cancelled);
        assert_eq!(run.steps[1].status, StepStatus::Skipped);
    }

    #[tokio::test]
    async fn unknown_variable_fails_before_running() {
        let (job, state) = job(
            pipeline(vec![local("one", "echo {{nope}}")]),
            BTreeMap::new(),
        );
        start(job);
        wait_finished(&state, 1).await;
        let run = state
            .read()
            .ok()
            .and_then(|s| s.pipeline_runs.first().cloned());
        assert!(matches!(run.map(|r| r.status), Some(RunStatus::Failed(_))));
    }

    async fn wait_finished(state: &SharedState, id: u64) {
        for _ in 0..200 {
            let running = state.read().ok().and_then(|s| {
                s.pipeline_runs
                    .iter()
                    .find(|r| r.id == id)
                    .map(|r| r.is_running())
            });
            if running == Some(false) {
                return;
            }
            tokio::time::sleep(Duration::from_millis(25)).await;
        }
    }
}
