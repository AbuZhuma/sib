mod events;
mod model;
mod parse;
mod run;
mod script;

use asiba_core::{
    Availability, CollectContext, CustomCheck, Module, ModuleError, ModuleId, ModuleSettings,
    Sample, Schedule, Snapshot, Transport,
};
use async_trait::async_trait;

pub use model::{CheckOutcome, CheckResult, ChecksSnapshot};

use crate::common::detect;

pub const ID: ModuleId = ModuleId("checks");
pub const KEY_FAILED: &str = "checks.failed";

const DETECT: &str = "command -v timeout";
const NO_TIMEOUT: &str = "нет timeout";

pub fn metric_key(check: &CustomCheck) -> String {
    format!("checks.{}.failed", check.id)
}

pub fn runnable(checks: &[CustomCheck]) -> Vec<CustomCheck> {
    checks
        .iter()
        .filter(|check| check.enabled && check.is_complete())
        .cloned()
        .collect()
}

pub struct ChecksModule;

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
        detect::require(transport, DETECT, NO_TIMEOUT).await
    }

    async fn collect(
        &self,
        transport: &dyn Transport,
        context: &CollectContext,
    ) -> Result<Snapshot, ModuleError> {
        let checks = runnable(&context.checks);
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
