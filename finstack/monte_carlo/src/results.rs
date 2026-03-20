//! Pricing-side Monte Carlo result types with explicit currency tags.
//!
//! This module wraps the numeric estimates from [`crate::estimate`] into
//! currency-aware result types for pricing APIs. All amounts here refer to
//! discounted path values unless a field explicitly says otherwise.

use crate::paths::PathDataset;
use finstack_core::currency::Currency;
use finstack_core::money::Money;
use serde::{Deserialize, Serialize};

/// Discounted Monte Carlo estimate tagged with a currency.
///
/// The engine computes these values from discounted path outcomes. `mean` and
/// `ci_95` are stored as [`Money`], while the auxiliary statistics remain raw
/// `f64` values in the same currency unit as `mean.amount()`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MoneyEstimate {
    /// Discounted mean present value.
    pub mean: Money,
    /// Standard error of the discounted mean, in `mean.amount()` units.
    pub stderr: f64,
    /// 95% confidence interval for the discounted mean present value.
    pub ci_95: (Money, Money),
    /// Number of simulated paths contributing to the estimate.
    pub num_paths: usize,
    /// Optional sample standard deviation of discounted path values.
    #[serde(default)]
    pub std_dev: Option<f64>,
    /// Optional median of captured discounted path values.
    ///
    /// This is populated only when captured-path diagnostics are available.
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

impl MoneyEstimate {
    /// Convert a numeric estimate into a currency-aware pricing estimate.
    ///
    /// # Arguments
    ///
    /// * `estimate` - Numeric Monte Carlo estimate on discounted path values.
    /// * `currency` - Currency tag to attach to `mean` and `ci_95`.
    ///
    /// # Returns
    ///
    /// A [`MoneyEstimate`] whose raw `f64` diagnostics remain in the same
    /// currency unit as `mean.amount()`.
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

    /// Return `stderr / abs(mean.amount())`.
    ///
    /// Returns `f64::INFINITY` when the estimate is numerically close to zero.
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

/// Monte Carlo pricing result with optional captured paths.
///
/// The estimate always reflects all simulated discounted path values used by the
/// pricing run. When `paths` is present, it contains the captured subset only.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MonteCarloResult {
    /// Discounted pricing estimate for the full simulation.
    pub estimate: MoneyEstimate,
    /// Optional captured-path subset for diagnostics and visualization.
    pub paths: Option<PathDataset>,
}

/// Container for simulation-based Greeks reported by higher-level APIs.
///
/// The generic engine in this module does not populate this struct directly.
/// Downstream pricers or bindings may use it as a convenience result type.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct MonteCarloGreeks {
    /// Delta estimate in price units per unit change in spot.
    pub delta: Option<f64>,
    /// Vega estimate. Individual estimator docs define the quoting convention.
    pub vega: Option<f64>,
}

impl MonteCarloResult {
    /// Create a result that only contains the aggregate estimate.
    pub fn new(estimate: MoneyEstimate) -> Self {
        Self {
            estimate,
            paths: None,
        }
    }

    /// Create a result that includes captured-path diagnostics.
    pub fn with_paths(estimate: MoneyEstimate, paths: PathDataset) -> Self {
        Self {
            estimate,
            paths: Some(paths),
        }
    }

    /// Return `true` when captured-path diagnostics are present.
    pub fn has_paths(&self) -> bool {
        self.paths.is_some()
    }

    /// Borrow the captured-path subset if available.
    pub fn paths(&self) -> Option<&PathDataset> {
        self.paths.as_ref()
    }

    /// Return the number of captured paths, or `0` when none were retained.
    pub fn num_captured_paths(&self) -> usize {
        self.paths.as_ref().map(|p| p.num_captured()).unwrap_or(0)
    }

    /// Borrow the aggregate discounted estimate.
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
