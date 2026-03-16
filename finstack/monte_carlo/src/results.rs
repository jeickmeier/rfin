//! Pricing-side Monte Carlo result types with currency.

use crate::paths::PathDataset;
use finstack_core::currency::Currency;
use finstack_core::money::Money;
use serde::{Deserialize, Serialize};

/// Estimate with currency information.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MoneyEstimate {
    /// Point estimate
    pub mean: Money,
    /// Standard error (in currency amount)
    pub stderr: f64,
    /// 95% confidence interval (lower, upper)
    pub ci_95: (Money, Money),
    /// Number of paths
    pub num_paths: usize,
    /// Optional: sample standard deviation
    #[serde(default)]
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

impl MoneyEstimate {
    /// Create from estimate and currency.
    pub fn from_estimate(estimate: crate::estimate::Estimate, currency: Currency) -> Self {
        Self {
            mean: Money::new(estimate.mean, currency),
            stderr: estimate.stderr,
            ci_95: (
                Money::new(estimate.ci_95.0, currency),
                Money::new(estimate.ci_95.1, currency),
            ),
            num_paths: estimate.num_paths,
            std_dev: estimate.std_dev,
            median: estimate.median,
            percentile_25: estimate.percentile_25,
            percentile_75: estimate.percentile_75,
            min: estimate.min,
            max: estimate.max,
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

/// Monte Carlo result with optional path data.
///
/// This structure wraps the statistical estimate along with optionally captured
/// paths for visualization and debugging.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MonteCarloResult {
    /// Statistical estimate (mean, stderr, CI)
    pub estimate: MoneyEstimate,
    /// Optional captured paths
    pub paths: Option<PathDataset>,
}

/// Monte Carlo Greeks (subset) computed via simulation-based estimators.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct MonteCarloGreeks {
    /// Delta estimate
    pub delta: Option<f64>,
    /// Vega estimate
    pub vega: Option<f64>,
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

    /// Get a reference to captured paths if available.
    pub fn paths(&self) -> Option<&PathDataset> {
        self.paths.as_ref()
    }

    /// Get the number of captured paths.
    pub fn num_captured_paths(&self) -> usize {
        self.paths.as_ref().map(|p| p.num_captured()).unwrap_or(0)
    }

    /// Get a reference to the estimate.
    pub fn estimate(&self) -> &MoneyEstimate {
        &self.estimate
    }
}

#[cfg(test)]
#[allow(clippy::expect_used, clippy::panic)]
mod tests {
    use super::*;
    use crate::estimate::Estimate;

    #[test]
    fn test_from_estimate_preserves_optional_diagnostics() {
        let estimate = Estimate::new(100.0, 1.0, (98.0, 102.0), 10_000)
            .with_std_dev(10.0)
            .with_median(99.0)
            .with_percentiles(95.0, 105.0)
            .with_range(80.0, 120.0);

        let money_estimate = MoneyEstimate::from_estimate(estimate, Currency::USD);

        assert_eq!(money_estimate.std_dev, Some(10.0));
        assert_eq!(money_estimate.median, Some(99.0));
        assert_eq!(money_estimate.percentile_25, Some(95.0));
        assert_eq!(money_estimate.percentile_75, Some(105.0));
        assert_eq!(money_estimate.min, Some(80.0));
        assert_eq!(money_estimate.max, Some(120.0));
    }
}
