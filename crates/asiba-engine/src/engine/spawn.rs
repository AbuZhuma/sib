use std::collections::HashMap;
use std::sync::atomic::AtomicBool;
use std::sync::{Arc, Mutex};

use tokio::sync::{mpsc, watch};

use super::{Engine, EngineDeps, EngineHandle, RepaintNotifier};
use crate::ai::{self, AuditWorker, Cancellations, StartupSummary};
use crate::alerts::{self, AlertLoop};
use crate::command::{Command, EngineEvent};
use crate::geo;
use crate::peers;

struct Channels {
    commands: mpsc::UnboundedSender<Command>,
    events: mpsc::UnboundedSender<EngineEvent>,
    alert_settings: watch::Receiver<alerts::AlertSettings>,
    ai_config: watch::Receiver<asiba_config::AiConfig>,
}

struct Background {
    alert_loop: AlertLoop,
    startup_summary: StartupSummary,
    audit_worker: AuditWorker,
}

pub fn spawn(
    runtime: &tokio::runtime::Handle,
    deps: EngineDeps,
    notify: RepaintNotifier,
) -> EngineHandle {
    let (commands, receiver) = mpsc::unbounded_channel();
    let (events, event_receiver) = mpsc::unbounded_channel();
    let (alert_settings, alert_receiver) = watch::channel(deps.alert_settings.clone());
    let (ai_config, ai_receiver) = watch::channel(deps.ai.clone());
    let cancellations = Cancellations::default();
    let channels = Channels {
        commands: commands.clone(),
        events: events.clone(),
        alert_settings: alert_receiver,
        ai_config: ai_receiver,
    };
    let background = background(&deps, &notify, channels, cancellations.clone());
    if let Ok(mut state) = deps.state.write() {
        state.ignored_incidents = deps.ignored.clone();
    }
    let engine = Engine {
        registry: deps.registry,
        state: deps.state,
        persistence: deps.persistence,
        storage: deps.storage,
        history_path: deps.history_path,
        geo_cache: deps.geo_cache,
        geolocation: Arc::new(AtomicBool::new(deps.geolocation)),
        intervals: deps.intervals.clamped(),
        notify,
        events,
        workers: HashMap::new(),
        alert_settings,
        ai_config,
        audits: ai::spawn_detached(),
        cancellations,
    };
    runtime.spawn(run(engine, receiver, background));
    EngineHandle {
        commands,
        events: Mutex::new(event_receiver),
    }
}

fn background(
    deps: &EngineDeps,
    notify: &RepaintNotifier,
    channels: Channels,
    cancellations: Cancellations,
) -> Background {
    Background {
        alert_loop: AlertLoop {
            state: Arc::clone(&deps.state),
            settings: channels.alert_settings,
            ai: channels.ai_config.clone(),
            notify: Arc::clone(notify),
            events: channels.events.clone(),
            commands: channels.commands.clone(),
        },
        startup_summary: StartupSummary {
            state: Arc::clone(&deps.state),
            config: channels.ai_config.clone(),
            commands: channels.commands,
        },
        audit_worker: AuditWorker {
            cancellations,
            state: Arc::clone(&deps.state),
            registry: deps.registry.clone(),
            config: channels.ai_config,
            audits_dir: deps.audits_dir.clone(),
            events: channels.events,
            notify: Arc::clone(notify),
        },
    }
}

async fn run(
    mut engine: Engine,
    mut receiver: mpsc::UnboundedReceiver<Command>,
    background: Background,
) {
    engine.audits = ai::spawn(background.audit_worker);
    ai::startup::spawn(background.startup_summary);
    alerts::spawn(background.alert_loop);
    geo::resolve_self(engine.geo_request());
    peers::spawn(peers::PeerLookup {
        cache: engine.geo_cache.clone(),
        enabled: Arc::clone(&engine.geolocation),
        state: Arc::clone(&engine.state),
        notify: Arc::clone(&engine.notify),
    });
    engine.load_action_journal();
    engine.load_saved().await;
    while let Some(command) = receiver.recv().await {
        engine.handle(command).await;
    }
}
