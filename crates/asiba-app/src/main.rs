use std::sync::Arc;

use anyhow::Context;
use asiba_config::{AppConfig, IgnoredStore, KeyringSecretStore, Paths, SecretStore, ServerStore};
use asiba_core::{AppState, Retention};
use asiba_engine::{AlertSettings, EngineDeps, Persistence};
use asiba_storage::{Database, spawn_writer};
use asiba_ui::AppDeps;
use tracing_subscriber::EnvFilter;

fn main() -> anyhow::Result<()> {
    init_tracing();
    let paths = Paths::discover().context("пути приложения")?;
    let mut config = AppConfig::load(&paths).context("config.toml")?;
    let secrets: Arc<dyn SecretStore> = Arc::new(KeyringSecretStore);
    if let Err(error) = config.load_ai_key(secrets.as_ref(), &paths) {
        tracing::warn!(%error, "ключ ИИ недоступен в keyring");
    }
    let servers = ServerStore::new(config.servers_dir(&paths));
    let persistence = Persistence::new(servers, Arc::clone(&secrets));
    let registry = asiba_modules::default_registry();
    let state = AppState::shared();
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .thread_name("asiba-engine")
        .build()
        .context("tokio runtime")?;
    let handle = runtime.handle().clone();
    let engine_state = Arc::clone(&state);
    let storage = open_storage(&paths, config.retention);
    let history_path = storage.is_some().then(|| paths.history_db());
    let alert_settings =
        AlertSettings::from_custom(&config.alert_rules, config.desktop_notifications);
    let engine_deps = EngineDeps {
        registry,
        state: engine_state,
        persistence,
        storage,
        history_path,
        geo_cache: Some(paths.geo_cache()),
        geolocation: config.geolocation,
        alert_settings,
        intervals: config.intervals,
        ai: config.ai.clone(),
        audits_dir: paths.audits_dir(),
        ignored: IgnoredStore::load(&paths).incidents,
    };
    let factory = Box::new(move |notify| asiba_engine::spawn(&handle, engine_deps, notify));
    let deps = AppDeps {
        state,
        paths,
        config,
        secrets,
    };
    asiba_ui::run(deps, factory).map_err(|e| anyhow::anyhow!("{e}"))?;
    runtime.shutdown_background();
    Ok(())
}

fn open_storage(paths: &Paths, retention: Retention) -> Option<asiba_storage::StorageWriter> {
    match Database::open(&paths.history_db()) {
        Ok(database) => Some(spawn_writer(database, retention)),
        Err(error) => {
            tracing::error!(%error, "история метрик отключена");
            None
        }
    }
}

fn init_tracing() {
    let filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info"));
    tracing_subscriber::fmt()
        .with_env_filter(filter)
        .with_target(false)
        .init();
}
