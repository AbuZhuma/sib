use std::any::Any;
use std::fmt::Debug;
use std::sync::Arc;

use chrono::{DateTime, Utc};

use crate::event::Event;

pub trait ModuleData: Any + Debug + Send + Sync {
    fn as_any(&self) -> &dyn Any;
}

impl<T: Any + Debug + Send + Sync> ModuleData for T {
    fn as_any(&self) -> &dyn Any {
        self
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Sample {
    pub key: String,
    pub value: f64,
}

impl Sample {
    pub fn new(key: impl Into<String>, value: f64) -> Self {
        Self {
            key: key.into(),
            value,
        }
    }
}

#[derive(Debug, Clone)]
pub struct Snapshot {
    pub taken_at: DateTime<Utc>,
    pub data: Arc<dyn ModuleData>,
    pub events: Vec<Event>,
    pub samples: Vec<Sample>,
}

impl Snapshot {
    pub fn new<T: ModuleData>(data: T) -> Self {
        Self {
            taken_at: Utc::now(),
            data: Arc::new(data),
            events: Vec::new(),
            samples: Vec::new(),
        }
    }

    pub fn with_events(mut self, events: Vec<Event>) -> Self {
        self.events = events;
        self
    }

    pub fn with_samples(mut self, samples: Vec<Sample>) -> Self {
        self.samples = samples;
        self
    }

    pub fn downcast<T: ModuleData>(&self) -> Option<&T> {
        self.data.as_ref().as_any().downcast_ref::<T>()
    }
}

#[derive(Debug, Clone, Default)]
pub struct CollectContext {
    pub previous: Option<Snapshot>,
    pub host: String,
}

impl CollectContext {
    pub fn previous<T: ModuleData>(&self) -> Option<(&T, f64)> {
        let snapshot = self.previous.as_ref()?;
        let elapsed = (Utc::now() - snapshot.taken_at).num_milliseconds() as f64 / 1000.0;
        if elapsed <= 0.0 {
            return None;
        }
        Some((snapshot.downcast::<T>()?, elapsed))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn downcast_returns_original_type() {
        let snapshot = Snapshot::new(String::from("data"));
        assert_eq!(
            snapshot.downcast::<String>().map(String::as_str),
            Some("data")
        );
    }

    #[test]
    fn previous_with_elapsed_time_returns_data() {
        let mut snapshot = Snapshot::new(42u32);
        snapshot.taken_at -= chrono::Duration::seconds(2);
        let context = CollectContext {
            previous: Some(snapshot),
            host: String::new(),
        };
        let (value, elapsed) = context.previous::<u32>().expect("previous");
        assert_eq!(*value, 42);
        assert!(elapsed >= 2.0);
    }
}
