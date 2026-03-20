//! Numeric Monte Carlo estimation results without currency tagging.
//!
//! [`Estimate`] is the engine's internal numeric summary for discounted path
//! values. Pricing-facing APIs usually convert it into
//! [`crate::results::MoneyEstimate`] once the output currency is known.

use serde::{Deserialize, Serialize};

/// Numeric Monte Carlo estimate for discounted path values.
///
/// All fields are unitless `f64` values in the same numeric unit as the
/// simulated discounted payoff. Pricing APIs usually wrap the mean and
/// confidence interval in [`finstack_core::money::Money`] after choosing a
/// currency.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Estimate {
    /// Mean of the discounted path values.
    pub mean: f64,
    /// Standard error of the discounted mean.
    pub stderr: f64,
    /// 95% confidence interval for the discounted mean.
    pub ci_95: (f64, f64),
    /// Number of simulated paths contributing to the estimate.
    pub num_paths: usize,
    /// Optional sample standard deviation of discounted path values.
    pub std_dev: Option<f64>,
    /// Optional median of captured discounted path values.
    #[serde(default)]
    pub median: Option<f64>,
    /// Optional 25th percentile of captured discounted path values.
    #[serde(default)]
    pub percentile_25: Option<f64>,
    /// Optional 75th percentile of captured discounted path values.
    #[serde(default)]
    pub percentile_75: Option<f64>,
    /// Optional minimum of captured discounted path values.
    #[serde(default)]
    pub min: Option<f64>,
    /// Optional maximum of captured discounted path values.
    #[serde(default)]
    pub max: Option<f64>,
}

impl Estimate {
    /// Create a new estimate from aggregate simulation statistics.
    ///
    /// # Arguments
    ///
    /// * `mean` - Discounted sample mean.
    /// * `stderr` - Standard error of the discounted mean.
    /// * `ci_95` - Lower and upper bounds of the 95% confidence interval.
    /// * `num_paths` - Number of simulated paths used to compute the estimate.
    ///
    /// # Returns
    ///
    /// An estimate without optional distribution diagnostics populated.
    pub fn new(mean: f64, stderr: f64, ci_95: (f64, f64), num_paths: usize) -> Self {
        Self {
            mean,
            stderr,
            ci_95,
            num_paths,
            std_dev: None,
            median: None,
            percentile_25: None,
            percentile_75: None,
            min: None,
            max: None,
        }
    }

    /// Attach the sample standard deviation of discounted path values.
    pub fn with_std_dev(mut self, std_dev: f64) -> Self {
        self.std_dev = Some(std_dev);
        self
    }

    /// Attach the median of captured discounted path values.
    pub fn with_median(mut self, median: f64) -> Self {
        self.median = Some(median);
        self
    }

    /// Attach the 25th and 75th percentiles of captured discounted path values.
    pub fn with_percentiles(mut self, p25: f64, p75: f64) -> Self {
        self.percentile_25 = Some(p25);
        self.percentile_75 = Some(p75);
        self
    }

    /// Attach the minimum and maximum of captured discounted path values.
    pub fn with_range(mut self, min: f64, max: f64) -> Self {
        self.min = Some(min);
        self.max = Some(max);
        self
    }

    /// Return the interquartile range when captured percentiles are available.
    pub fn iqr(&self) -> Option<f64> {
        match (self.percentile_25, self.percentile_75) {
            (Some(p25), Some(p75)) => Some(p75 - p25),
            _ => None,
        }
    }

    /// Return `max - min` when captured extrema are available.
    pub fn range(&self) -> Option<f64> {
        match (self.min, self.max) {
            (Some(min), Some(max)) => Some(max - min),
            _ => None,
        }
    }

    /// Return `stderr / abs(mean)`.
    ///
    /// Returns `f64::INFINITY` when the estimate is numerically close to zero.
    pub fn relative_stderr(&self) -> f64 {
        if self.mean.abs() < 1e-10 {
            f64::INFINITY
        } else {
            self.stderr.abs() / self.mean.abs()
        }
    }

    /// Return the coefficient of variation `std_dev / abs(mean)` when available.
    pub fn cv(&self) -> Option<f64> {
        self.std_dev.map(|sd| {
            if self.mean.abs() < 1e-10 {
                f64::INFINITY
            } else {
                sd.abs() / self.mean.abs()
            }
        })
    }

    /// Return half the width of `ci_95`.
    pub fn ci_half_width(&self) -> f64 {
        (self.ci_95.1 - self.ci_95.0) / 2.0
    }
}

impl std::fmt::Display for Estimate {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{:.6} ± {:.6} [{:.6}, {:.6}] (n={})",
            self.mean, self.stderr, self.ci_95.0, self.ci_95.1, self.num_paths
        )
    }
}

/// Compatibility type for bindings that still expose Monte Carlo diagnostics.
///
/// The engine does not currently populate this structure, but it remains part of
/// the public API while downstream bindings and stubs still import it.
///
/// All fields are currently placeholders and remain `None` unless a downstream
/// caller fills them manually. Prefer the statistics on [`Estimate`] and any
/// simulation-result summaries for actual convergence analysis.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConvergenceDiagnostics {
    /// Stderr decay rate (should be ~-0.5 for MC)
    pub stderr_decay_rate: Option<f64>,
    /// Effective sample size (for weighted samples)
    pub effective_sample_size: Option<usize>,
    /// Variance reduction factor (vs. baseline)
    pub variance_reduction_factor: Option<f64>,
}

impl ConvergenceDiagnostics {
    /// Create empty compatibility diagnostics.
    ///
    /// The Monte Carlo engine does not populate these fields today, so new
    /// instances start empty and typically remain empty.
    pub fn new() -> Self {
        Self {
            stderr_decay_rate: None,
            effective_sample_size: None,
            variance_reduction_factor: None,
        }
    }

    /// Set the observed stderr decay rate.
    pub fn with_stderr_decay(mut self, rate: f64) -> Self {
        self.stderr_decay_rate = Some(rate);
        self
    }

    /// Set the effective sample size.
    pub fn with_ess(mut self, ess: usize) -> Self {
        self.effective_sample_size = Some(ess);
        self
    }

    /// Set the variance-reduction factor versus a baseline run.
    pub fn with_vr_factor(mut self, factor: f64) -> Self {
        self.variance_reduction_factor = Some(factor);
        self
    }
}

impl Default for ConvergenceDiagnostics {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
#[allow(clippy::expect_used, clippy::panic)]
mod tests {
    use super::*;

    #[test]
    fn test_estimate_creation() {
        let est = Estimate::new(100.0, 1.0, (98.0, 102.0), 10000);
        assert_eq!(est.mean, 100.0);
        assert_eq!(est.stderr, 1.0);
        assert_eq!(est.num_paths, 10000);
        assert_eq!(est.ci_half_width(), 2.0);
    }

    #[test]
    fn test_relative_stderr() {
        let est = Estimate::new(100.0, 1.0, (98.0, 102.0), 10000);
        assert_eq!(est.relative_stderr(), 0.01); // 1%
    }

    // MoneyEstimate tests moved with pricing module

    #[test]
    fn test_display() {
        let est = Estimate::new(100.0, 1.0, (98.0, 102.0), 10000);
        let s = format!("{}", est);
        assert!(s.contains("100."));
        assert!(s.contains("n=10000"));
    }

    // MonteCarloResult tests moved with pricing module
}
