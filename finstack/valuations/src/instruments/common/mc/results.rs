//! Monte Carlo estimation results.
//!
//! Provides structured results with mean, standard error, confidence intervals,
//! and metadata for Monte Carlo simulations.

use super::path_data::PathDataset;
use finstack_core::currency::Currency;
use finstack_core::money::Money;
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
        }
    }

    /// Create estimate with standard deviation.
    pub fn with_std_dev(mut self, std_dev: f64) -> Self {
        self.std_dev = Some(std_dev);
        self
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

    /// Convert to Money with specified currency.
    pub fn to_money(&self, currency: Currency) -> Money {
        Money::new(self.mean, currency)
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

/// Estimate with currency information.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct MoneyEstimate {
    /// Point estimate
    pub mean: Money,
    /// Standard error (in currency amount)
    pub stderr: f64,
    /// 95% confidence interval (lower, upper)
    pub ci_95: (Money, Money),
    /// Number of paths
    pub num_paths: usize,
}

impl MoneyEstimate {
    /// Create from estimate and currency.
    pub fn from_estimate(estimate: Estimate, currency: Currency) -> Self {
        Self {
            mean: Money::new(estimate.mean, currency),
            stderr: estimate.stderr,
            ci_95: (
                Money::new(estimate.ci_95.0, currency),
                Money::new(estimate.ci_95.1, currency),
            ),
            num_paths: estimate.num_paths,
        }
    }

    /// Relative standard error.
    pub fn relative_stderr(&self) -> f64 {
        if self.mean.amount().abs() < 1e-10 {
            f64::INFINITY
        } else {
            self.stderr.abs() / self.mean.amount().abs()
        }
    }
}

impl std::fmt::Display for MoneyEstimate {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{} ± {:.6} [{}, {}] (n={})",
            self.mean, self.stderr, self.ci_95.0, self.ci_95.1, self.num_paths
        )
    }
}

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

/// Monte Carlo result with optional path data.
///
/// This structure wraps the statistical estimate along with optionally captured
/// paths for visualization and debugging.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct MonteCarloResult {
    /// Statistical estimate (mean, stderr, CI)
    pub estimate: MoneyEstimate,
    /// Optional captured paths
    pub paths: Option<PathDataset>,
}

impl MonteCarloResult {
    /// Create a new Monte Carlo result without paths.
    pub fn new(estimate: MoneyEstimate) -> Self {
        Self {
            estimate,
            paths: None,
        }
    }

    /// Create a Monte Carlo result with paths.
    pub fn with_paths(estimate: MoneyEstimate, paths: PathDataset) -> Self {
        Self {
            estimate,
            paths: Some(paths),
        }
    }

    /// Check if paths were captured.
    pub fn has_paths(&self) -> bool {
        self.paths.is_some()
    }

    /// Get the number of captured paths.
    pub fn num_captured_paths(&self) -> usize {
        self.paths.as_ref().map(|p| p.num_captured()).unwrap_or(0)
    }

    /// Get a reference to the estimate.
    pub fn estimate(&self) -> &MoneyEstimate {
        &self.estimate
    }

    /// Get a reference to the paths (if available).
    pub fn paths(&self) -> Option<&PathDataset> {
        self.paths.as_ref()
    }

    /// Consume self and return the estimate and paths separately.
    pub fn into_parts(self) -> (MoneyEstimate, Option<PathDataset>) {
        (self.estimate, self.paths)
    }
}

impl std::fmt::Display for MonteCarloResult {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.estimate)?;
        if let Some(ref paths) = self.paths {
            write!(
                f,
                " [captured {}/{} paths]",
                paths.num_captured(),
                paths.num_paths_total
            )?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use finstack_core::currency::Currency;

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

    #[test]
    fn test_money_estimate() {
        let est = Estimate::new(100.0, 1.0, (98.0, 102.0), 10000);
        let money_est = MoneyEstimate::from_estimate(est, Currency::USD);
        assert_eq!(money_est.mean.currency(), Currency::USD);
        assert_eq!(money_est.mean.amount(), 100.0);
    }

    #[test]
    fn test_display() {
        let est = Estimate::new(100.0, 1.0, (98.0, 102.0), 10000);
        let s = format!("{}", est);
        assert!(s.contains("100."));
        assert!(s.contains("n=10000"));
    }

    #[test]
    fn test_monte_carlo_result() {
        let est = Estimate::new(100.0, 1.0, (98.0, 102.0), 10000);
        let money_est = MoneyEstimate::from_estimate(est, Currency::USD);

        let result = MonteCarloResult::new(money_est.clone());
        assert!(!result.has_paths());
        assert_eq!(result.num_captured_paths(), 0);
        assert_eq!(result.estimate().mean.amount(), 100.0);
    }
}
