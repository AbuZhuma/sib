use std::collections::VecDeque;

use chrono::{DateTime, Duration, Utc};

pub const DEFAULT_CAPACITY: usize = 900;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Point {
    pub at: DateTime<Utc>,
    pub value: f64,
}

#[derive(Debug, Clone)]
pub struct Series {
    points: VecDeque<Point>,
    capacity: usize,
}

impl Default for Series {
    fn default() -> Self {
        Self::with_capacity(DEFAULT_CAPACITY)
    }
}

impl Series {
    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            points: VecDeque::with_capacity(capacity),
            capacity,
        }
    }

    pub fn push(&mut self, point: Point) {
        if self.points.len() == self.capacity {
            self.points.pop_front();
        }
        self.points.push_back(point);
    }

    pub fn latest(&self) -> Option<Point> {
        self.points.back().copied()
    }

    pub fn iter(&self) -> impl Iterator<Item = Point> + '_ {
        self.points.iter().copied()
    }

    pub fn since(&self, window: Duration) -> impl Iterator<Item = Point> + '_ {
        let from = Utc::now() - window;
        self.points.iter().copied().filter(move |p| p.at >= from)
    }

    pub fn len(&self) -> usize {
        self.points.len()
    }

    pub fn is_empty(&self) -> bool {
        self.points.is_empty()
    }

    pub fn max_value(&self) -> f64 {
        self.points.iter().map(|p| p.value).fold(0.0, f64::max)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn point(value: f64) -> Point {
        Point {
            at: Utc::now(),
            value,
        }
    }

    #[test]
    fn push_beyond_capacity_drops_oldest() {
        let mut series = Series::with_capacity(2);
        series.push(point(1.0));
        series.push(point(2.0));
        series.push(point(3.0));
        let values: Vec<f64> = series.iter().map(|p| p.value).collect();
        assert_eq!(values, vec![2.0, 3.0]);
    }

    #[test]
    fn latest_on_empty_is_none() {
        assert_eq!(Series::default().latest(), None);
    }

    #[test]
    fn max_value_returns_largest() {
        let mut series = Series::default();
        series.push(point(4.0));
        series.push(point(9.5));
        assert_eq!(series.max_value(), 9.5);
    }
}
