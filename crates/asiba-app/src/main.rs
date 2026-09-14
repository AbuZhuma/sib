use std::sync::Arc;

use anyhow::Context;
use asiba_config::{AppConfig, KeyringSecretStore, Paths, ServerStore};
use asiba_core::AppState;
use asiba_engine::Persistence;
use asiba_ui::AppDeps;
use tracing_subscriber::EnvFilter;

fn main() -> anyhow::Result<()> {
    init_tracing();
    let paths = Paths::discover().context("пути приложения")?;
    let config = AppConfig::load(&paths).context("config.toml")?;
    let servers = ServerStore::new(config.servers_dir(&paths));
    let persistence = Persistence::new(servers, Arc::new(KeyringSecretStore));
    let registry = asiba_modules::default_registry();
    let state = AppState::shared();
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .thread_name("asiba-engine")
        .build()
        .context("tokio runtime")?;
    let handle = runtime.handle().clone();
    let engine_state = Arc::clone(&state);
    let factory = Box::new(move |notify| {
        asiba_engine::spawn(&handle, registry, engine_state, persistence, notify)
    });
    let deps = AppDeps {
        state,
        paths,
        config,
    };
    asiba_ui::run(deps, factory).map_err(|e| anyhow::anyhow!("{e}"))?;
    runtime.shutdown_background();
    Ok(())
}

fn init_tracing() {
    let filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info"));
    tracing_subscriber::fmt()
        .with_env_filter(filter)
        .with_target(false)
        .init();
}
