pub fn per_second(current: u64, previous: u64, elapsed_secs: f64) -> f64 {
    if elapsed_secs <= 0.0 || current < previous {
        return 0.0;
    }
    (current - previous) as f64 / elapsed_secs
}

pub fn percent(part: u64, whole: u64) -> f64 {
    if whole == 0 {
        return 0.0;
    }
    part as f64 * 100.0 / whole as f64
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn per_second_divides_delta_by_elapsed() {
        assert_eq!(per_second(300, 100, 2.0), 100.0);
    }

    #[test]
    fn per_second_counter_reset_gives_zero() {
        assert_eq!(per_second(10, 100, 2.0), 0.0);
    }

    #[test]
    fn percent_of_zero_whole_is_zero() {
        assert_eq!(percent(5, 0), 0.0);
    }
}
