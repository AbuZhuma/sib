use std::time::Duration;

const STEPS: [u64; 4] = [5, 10, 30, 60];

#[derive(Debug, Default, Clone, Copy)]
pub struct Backoff {
    attempt: usize,
}

impl Backoff {
    pub fn next(&mut self) -> Duration {
        let index = self.attempt.min(STEPS.len() - 1);
        self.attempt += 1;
        Duration::from_secs(STEPS[index])
    }

    pub fn reset(&mut self) {
        self.attempt = 0;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn next_grows_then_caps_at_last_step() {
        let mut backoff = Backoff::default();
        let seconds: Vec<u64> = (0..6).map(|_| backoff.next().as_secs()).collect();
        assert_eq!(seconds, vec![5, 10, 30, 60, 60, 60]);
    }

    #[test]
    fn reset_returns_to_first_step() {
        let mut backoff = Backoff::default();
        backoff.next();
        backoff.next();
        backoff.reset();
        assert_eq!(backoff.next().as_secs(), 5);
    }
}
