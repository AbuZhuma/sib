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

#[derive(Debug, Clone)]
pub struct Snapshot {
    pub taken_at: DateTime<Utc>,
    pub data: Arc<dyn ModuleData>,
    pub events: Vec<Event>,
}

impl Snapshot {
    pub fn new<T: ModuleData>(data: T) -> Self {
        Self {
            taken_at: Utc::now(),
            data: Arc::new(data),
            events: Vec::new(),
        }
    }

    pub fn with_events(mut self, events: Vec<Event>) -> Self {
        self.events = events;
        self
    }

    pub fn downcast<T: ModuleData>(&self) -> Option<&T> {
        self.data.as_any().downcast_ref::<T>()
    }
}
