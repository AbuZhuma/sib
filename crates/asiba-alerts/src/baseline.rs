const HALF_LIFE_SAMPLES: f64 = 720.0;
const WARMUP_SAMPLES: u32 = 120;
const SIGMA_FACTOR: f64 = 4.0;
const MIN_SIGMA_RELATIVE: f64 = 0.05;
const MIN_SIGMA_ABSOLUTE: f64 = 1.0;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Baseline {
    mean: f64,
    variance: f64,
    samples: u32,
}

impl Default for Baseline {
    fn default() -> Self {
        Self {
            mean: 0.0,
            variance: 0.0,
            samples: 0,
        }
    }
}

impl Baseline {
    pub fn observe(&mut self, value: f64) -> Option<Deviation> {
        let deviation = self.deviation(value);
        let alpha = 1.0 - (-std::f64::consts::LN_2 / HALF_LIFE_SAMPLES).exp();
        if self.samples == 0 {
            self.mean = value;
        } else {
            let delta = value - self.mean;
            self.mean += alpha * delta;
            self.variance = (1.0 - alpha) * (self.variance + alpha * delta * delta);
        }
        self.samples = self.samples.saturating_add(1);
        deviation
    }

    fn deviation(&self, value: f64) -> Option<Deviation> {
        if self.samples < WARMUP_SAMPLES {
            return None;
        }
        let sigma = self
            .variance
            .sqrt()
            .max(self.mean.abs() * MIN_SIGMA_RELATIVE)
            .max(MIN_SIGMA_ABSOLUTE);
        let distance = (value - self.mean) / sigma;
        (distance.abs() >= SIGMA_FACTOR).then_some(Deviation {
            mean: self.mean,
            sigmas: distance,
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Deviation {
    pub mean: f64,
    pub sigmas: f64,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn warmed(value: f64) -> Baseline {
        let mut baseline = Baseline::default();
        for i in 0..WARMUP_SAMPLES {
            let jitter = if i % 2 == 0 { 1.0 } else { -1.0 };
            baseline.observe(value + jitter);
        }
        baseline
    }

    #[test]
    fn baseline_during_warmup_reports_nothing() {
        let mut baseline = Baseline::default();
        assert_eq!(baseline.observe(1000.0), None);
        assert_eq!(baseline.observe(0.0), None);
    }

    #[test]
    fn stable_value_after_warmup_is_not_deviation() {
        let mut baseline = warmed(20.0);
        assert_eq!(baseline.observe(21.0), None);
    }

    #[test]
    fn spike_after_warmup_is_deviation() {
        let mut baseline = warmed(20.0);
        let deviation = baseline.observe(95.0).expect("deviation");
        assert!(deviation.sigmas > SIGMA_FACTOR);
        assert!((deviation.mean - 20.0).abs() < 1.0);
    }
}
