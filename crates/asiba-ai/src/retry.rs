use std::time::Duration;

use crate::backend::{Backend, Completion};
use crate::error::AiError;

pub const RETRY_DELAYS: [Duration; 3] = [
    Duration::from_secs(3),
    Duration::from_secs(10),
    Duration::from_secs(30),
];

pub fn complete_with_retry(
    backend: &dyn Backend,
    request: &Completion,
    delays: &[Duration],
) -> Result<String, AiError> {
    let mut attempt = 0;
    loop {
        match backend.complete(request) {
            Err(error) if error.is_transient() && attempt < delays.len() => {
                tracing::warn!(
                    model = %backend.model_name(),
                    attempt,
                    "временная ошибка ИИ, повтор: {error}"
                );
                std::thread::sleep(delays[attempt]);
                attempt += 1;
            }
            result => return result,
        }
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Mutex;

    use super::*;

    struct Flaky {
        failures_left: Mutex<usize>,
        status: u16,
    }

    impl Backend for Flaky {
        fn model_name(&self) -> String {
            "flaky".to_owned()
        }

        fn complete(&self, _request: &Completion) -> Result<String, AiError> {
            let mut left = self.failures_left.lock().expect("lock");
            if *left == 0 {
                return Ok("ok".to_owned());
            }
            *left -= 1;
            Err(AiError::api(self.status, "busy"))
        }
    }

    fn request() -> Completion {
        Completion {
            system: String::new(),
            user: String::new(),
            max_tokens: 1,
        }
    }

    #[test]
    fn transient_error_is_retried_until_success() {
        let backend = Flaky {
            failures_left: Mutex::new(2),
            status: 503,
        };
        let delays = [Duration::ZERO; 3];
        assert_eq!(
            complete_with_retry(&backend, &request(), &delays).expect("ok"),
            "ok"
        );
    }

    #[test]
    fn permanent_error_is_not_retried() {
        let backend = Flaky {
            failures_left: Mutex::new(1),
            status: 401,
        };
        let delays = [Duration::ZERO; 3];
        assert!(complete_with_retry(&backend, &request(), &delays).is_err());
        assert_eq!(*backend.failures_left.lock().expect("lock"), 0);
    }

    #[test]
    fn retries_are_bounded_by_delay_count() {
        let backend = Flaky {
            failures_left: Mutex::new(5),
            status: 503,
        };
        let delays = [Duration::ZERO; 2];
        assert!(complete_with_retry(&backend, &request(), &delays).is_err());
        assert_eq!(*backend.failures_left.lock().expect("lock"), 2);
    }
}
