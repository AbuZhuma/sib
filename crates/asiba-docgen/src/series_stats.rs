use asiba_core::{Series, ServerState};
use chrono::Duration;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct WindowStats {
    pub average: f64,
    pub max: f64,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MetricStats {
    pub current: f64,
    pub hour: Option<WindowStats>,
    pub day: Option<WindowStats>,
}

pub const HOUR: Duration = Duration::hours(1);
pub const DAY: Duration = Duration::days(1);

pub fn window(series: &Series, window: Duration) -> Option<WindowStats> {
    let mut count = 0usize;
    let mut sum = 0.0;
    let mut max = f64::MIN;
    for point in series.since(window) {
        count += 1;
        sum += point.value;
        max = max.max(point.value);
    }
    if count == 0 {
        return None;
    }
    Some(WindowStats {
        average: sum / count as f64,
        max,
    })
}

pub fn metric(server: &ServerState, key: &str) -> Option<MetricStats> {
    let series = server.series.get(key)?;
    let current = series.latest()?.value;
    Some(MetricStats {
        current,
        hour: window(series, HOUR),
        day: window(series, DAY),
    })
}

#[cfg(test)]
mod tests {
    use asiba_core::Point;
    use chrono::Utc;

    use super::*;

    fn series(values: &[f64]) -> Series {
        let mut series = Series::with_capacity(values.len());
        let start = Utc::now() - Duration::minutes(values.len() as i64);
        for (index, value) in values.iter().enumerate() {
            series.push(Point {
                at: start + Duration::minutes(index as i64),
                value: *value,
            });
        }
        series
    }

    #[test]
    fn window_computes_average_and_max() {
        let stats = window(&series(&[10.0, 20.0, 60.0]), HOUR).expect("stats");
        assert_eq!(stats.average, 30.0);
        assert_eq!(stats.max, 60.0);
    }

    #[test]
    fn window_without_points_is_none() {
        assert!(window(&Series::with_capacity(0), HOUR).is_none());
    }
}
