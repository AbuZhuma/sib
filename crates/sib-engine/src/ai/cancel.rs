use std::collections::HashSet;
use std::sync::{Arc, Mutex};

use tokio::sync::Notify;

#[derive(Clone, Default)]
pub struct Cancellations {
    ids: Arc<Mutex<HashSet<u64>>>,
    signal: Arc<Notify>,
}

impl Cancellations {
    pub fn cancel(&self, id: u64) {
        if let Ok(mut ids) = self.ids.lock() {
            ids.insert(id);
        }
        self.signal.notify_one();
    }

    pub fn is_cancelled(&self, id: u64) -> bool {
        self.ids.lock().is_ok_and(|ids| ids.contains(&id))
    }

    pub fn take(&self, id: u64) -> bool {
        self.ids.lock().is_ok_and(|mut ids| ids.remove(&id))
    }

    pub async fn wait_for(&self, id: u64) {
        loop {
            if self.is_cancelled(id) {
                return;
            }
            self.signal.notified().await;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn wait_for_returns_after_cancel_of_the_same_id() {
        let cancellations = Cancellations::default();
        cancellations.cancel(7);
        cancellations.cancel(3);
        cancellations.wait_for(3).await;
        assert!(cancellations.take(3));
        assert!(!cancellations.take(3));
        assert!(cancellations.is_cancelled(7));
    }
}
