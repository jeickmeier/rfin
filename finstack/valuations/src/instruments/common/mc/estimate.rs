//! Monte Carlo estimation results.
//!
//! Provides structured results with mean, standard error, confidence intervals,
//! and metadata for Monte Carlo simulations.

use serde::{Deserialize, Serialize};

/// Monte Carlo estimation result.
///
/// Contains point estimate, uncertainty quantification, and metadata
/// about the simulation run.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Estimate {
    /// Point estimate (mean)
    pub mean: f64,
    /// Standard error of the mean
    pub stderr: f64,
    /// 95% confidence interval
    pub ci_95: (f64, f64),
    /// Number of paths simulated
    pub num_paths: usize,
    /// Optional: sample standard deviation
    pub std_dev: Option<f64>,
    /// Optional: median value
    #[serde(default)]
    pub median: Option<f64>,
    /// Optional: 25th percentile
    #[serde(default)]
    pub percentile_25: Option<f64>,
    /// Optional: 75th percentile
    #[serde(default)]
    pub percentile_75: Option<f64>,
    /// Optional: minimum value
    #[serde(default)]
    pub min: Option<f64>,
    /// Optional: maximum value
    #[serde(default)]
    pub max: Option<f64>,
}

impl Estimate {
    /// Create a new estimate.
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

    /// Create estimate with standard deviation.
    pub fn with_std_dev(mut self, std_dev: f64) -> Self {
        self.std_dev = Some(std_dev);
        self
    }

    /// Set median value.
    pub fn with_median(mut self, median: f64) -> Self {
        self.median = Some(median);
        self
    }

    /// Set percentiles (25th and 75th).
    pub fn with_percentiles(mut self, p25: f64, p75: f64) -> Self {
        self.percentile_25 = Some(p25);
        self.percentile_75 = Some(p75);
        self
    }

    /// Set min and max values.
    pub fn with_range(mut self, min: f64, max: f64) -> Self {
        self.min = Some(min);
        self.max = Some(max);
        self
    }

    /// Get interquartile range (IQR) if percentiles are available.
    pub fn iqr(&self) -> Option<f64> {
        match (self.percentile_25, self.percentile_75) {
            (Some(p25), Some(p75)) => Some(p75 - p25),
            _ => None,
        }
    }

    /// Get range (max - min) if available.
    pub fn range(&self) -> Option<f64> {
        match (self.min, self.max) {
            (Some(min), Some(max)) => Some(max - min),
            _ => None,
        }
    }

    /// Relative standard error (stderr / mean).
    pub fn relative_stderr(&self) -> f64 {
        if self.mean.abs() < 1e-10 {
            f64::INFINITY
        } else {
            self.stderr.abs() / self.mean.abs()
        }
    }

    /// Coefficient of variation (std_dev / mean).
    pub fn cv(&self) -> Option<f64> {
        self.std_dev.map(|sd| {
            if self.mean.abs() < 1e-10 {
                f64::INFINITY
            } else {
                sd.abs() / self.mean.abs()
            }
        })
    }

    /// Half-width of the 95% confidence interval.
    pub fn ci_half_width(&self) -> f64 {
        (self.ci_95.1 - self.ci_95.0) / 2.0
    }

    // Pricing-side currency conversion moved to models::monte_carlo::results
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

// MoneyEstimate moved to instruments::common::models::monte_carlo::results

/// Convergence diagnostics for Monte Carlo simulation.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ConvergenceDiagnostics {
    /// Stderr decay rate (should be ~-0.5 for MC)
    pub stderr_decay_rate: Option<f64>,
    /// Effective sample size (for weighted samples)
    pub effective_sample_size: Option<usize>,
    /// Variance reduction factor (vs. baseline)
    pub variance_reduction_factor: Option<f64>,
}

impl ConvergenceDiagnostics {
    /// Create empty diagnostics.
    pub fn new() -> Self {
        Self {
            stderr_decay_rate: None,
            effective_sample_size: None,
            variance_reduction_factor: None,
        }
    }

    /// With stderr decay rate.
    pub fn with_stderr_decay(mut self, rate: f64) -> Self {
        self.stderr_decay_rate = Some(rate);
        self
    }

    /// With effective sample size.
    pub fn with_ess(mut self, ess: usize) -> Self {
        self.effective_sample_size = Some(ess);
        self
    }

    /// With variance reduction factor.
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

// Monte Carlo result with optional path data.
//
// This structure wraps the statistical estimate along with optionally captured
// paths for visualization and debugging.
// MonteCarloResult moved to instruments::common::models::monte_carlo::results

// Display for MonteCarloResult moved with pricing module

#[cfg(test)]
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
