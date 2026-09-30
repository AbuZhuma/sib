mod events;
mod model;
mod parse;
mod run;
mod script;

use std::sync::{Arc, RwLock};

use asiba_core::{
    Availability, CollectContext, CustomCheck, Module, ModuleError, ModuleId, ModuleSettings,
    Sample, Schedule, Snapshot, SudoMode, Transport,
};
use async_trait::async_trait;

pub use model::{CheckOutcome, CheckResult, ChecksSnapshot};

use crate::common::detect;

pub const ID: ModuleId = ModuleId("checks");
pub const KEY_FAILED: &str = "checks.failed";

const DETECT: &str = "command -v timeout";
const NO_CHECKS: &str = "свои проверки не заданы";
const NO_TIMEOUT: &str = "нет timeout";
const NO_SUDO: &str = "проверки от root (sudo не настроен)";

pub fn metric_key(check: &CustomCheck) -> String {
    format!("checks.{}.failed", check.id)
}

#[derive(Clone, Default)]
pub struct Defined(Arc<RwLock<Vec<CustomCheck>>>);

impl Defined {
    pub fn new(checks: Vec<CustomCheck>) -> Self {
        Self(Arc::new(RwLock::new(checks)))
    }

    pub fn set(&self, checks: Vec<CustomCheck>) {
        if let Ok(mut current) = self.0.write() {
            *current = checks;
        }
    }

    pub fn runnable(&self) -> Vec<CustomCheck> {
        let Ok(checks) = self.0.read() else {
            return Vec::new();
        };
        checks
            .iter()
            .filter(|check| check.enabled && check.is_complete())
            .cloned()
            .collect()
    }
}

pub struct ChecksModule {
    defined: Defined,
}

impl ChecksModule {
    pub fn new(defined: Defined) -> Self {
        Self { defined }
    }
}

#[async_trait]
impl Module for ChecksModule {
    fn id(&self) -> ModuleId {
        ID
    }

    fn title(&self) -> &'static str {
        "Свои проверки"
    }

    fn schedule(&self) -> Schedule {
        Schedule::Slow
    }

    async fn detect(
        &self,
        transport: &dyn Transport,
        _settings: &ModuleSettings,
    ) -> Result<Availability, ModuleError> {
        let checks = self.defined.runnable();
        if checks.is_empty() {
            return Ok(Availability::unavailable(NO_CHECKS));
        }
        let available = detect::require(transport, DETECT, NO_TIMEOUT).await?;
        if !available.is_usable() {
            return Ok(available);
        }
        let needs_root = checks.iter().any(|check| check.as_root);
        if needs_root && transport.sudo_mode() == SudoMode::None {
            return Ok(Availability::partial(NO_SUDO));
        }
        Ok(available)
    }

    async fn collect(
        &self,
        transport: &dyn Transport,
        context: &CollectContext,
    ) -> Result<Snapshot, ModuleError> {
        let checks = self.defined.runnable();
        let results = run::all(transport, &checks).await?;
        let samples = samples(&results);
        let previous = context.previous.as_ref().and_then(Snapshot::downcast);
        let events = events::changes(previous, &results);
        Ok(Snapshot::new(ChecksSnapshot { results })
            .with_samples(samples)
            .with_events(events))
    }
}

fn samples(results: &[CheckResult]) -> Vec<Sample> {
    let mut samples: Vec<Sample> = results
        .iter()
        .map(|result| Sample::new(metric_key(&result.check), f64::from(result.is_failed())))
        .collect();
    samples.push(Sample::new(
        KEY_FAILED,
        results.iter().filter(|r| r.is_failed()).count() as f64,
    ));
    samples
}
