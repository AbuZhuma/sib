use std::collections::BTreeMap;
use std::time::Duration;

use asiba_core::ModuleId;
use tokio::time::Instant;

const REDETECT_INTERVAL: Duration = Duration::from_secs(600);

pub enum Due {
    Collect(ModuleId),
    Redetect,
}

pub struct Scheduler {
    due: BTreeMap<ModuleId, (Instant, Duration)>,
    redetect_at: Instant,
}

impl Scheduler {
    pub fn new(modules: impl IntoIterator<Item = (ModuleId, Duration)>) -> Self {
        let now = Instant::now();
        let due = modules
            .into_iter()
            .map(|(id, interval)| (id, (now, interval)))
            .collect();
        Self {
            due,
            redetect_at: now + REDETECT_INTERVAL,
        }
    }

    pub fn next(&self) -> (Instant, Due) {
        let earliest = self
            .due
            .iter()
            .min_by_key(|(_, (at, _))| *at)
            .map(|(id, (at, _))| (*at, Due::Collect(*id)));
        match earliest {
            Some((at, due)) if at <= self.redetect_at => (at, due),
            _ => (self.redetect_at, Due::Redetect),
        }
    }

    pub fn mark_collected(&mut self, id: ModuleId) {
        if let Some((at, interval)) = self.due.get_mut(&id) {
            *at = Instant::now() + *interval;
        }
    }

    pub fn request_redetect(&mut self) {
        self.redetect_at = Instant::now();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const A: ModuleId = ModuleId("a");
    const B: ModuleId = ModuleId("b");

    #[test]
    fn next_returns_earliest_module() {
        let mut scheduler =
            Scheduler::new([(A, Duration::from_secs(1)), (B, Duration::from_secs(100))]);
        scheduler.mark_collected(A);
        scheduler.mark_collected(B);
        assert!(matches!(scheduler.next().1, Due::Collect(id) if id == A));
    }

    #[test]
    fn next_without_modules_is_redetect() {
        let scheduler = Scheduler::new([]);
        assert!(matches!(scheduler.next().1, Due::Redetect));
    }
}
