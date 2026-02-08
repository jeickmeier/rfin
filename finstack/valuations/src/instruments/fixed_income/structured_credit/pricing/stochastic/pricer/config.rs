//! Stochastic pricer configuration.
#![allow(dead_code)] // Public API items may be used by external bindings

use finstack_core::dates::Date;
use finstack_core::market_data::term_structures::DiscountCurve;

use crate::instruments::fixed_income::structured_credit::pricing::stochastic::tree::ScenarioTreeConfig;
use std::sync::Arc;

/// Pricing mode selection.
#[derive(Debug, Clone, PartialEq, Eq, Default, serde::Serialize, serde::Deserialize)]
#[non_exhaustive]
pub enum PricingMode {
    /// Tree-based pricing (exact, non-recombining)
    #[default]
    Tree,
    /// Monte Carlo pricing with specified number of paths
    MonteCarlo {
        /// Number of simulation paths
        num_paths: usize,
        /// Use antithetic variates for variance reduction
        antithetic: bool,
    },
    /// Hybrid: tree for short horizons, MC for long
    Hybrid {
        /// Tree periods before switching to MC
        tree_periods: usize,
        /// MC paths for tail
        mc_paths: usize,
    },
}

impl PricingMode {
    /// Create tree pricing mode.
    pub fn tree() -> Self {
        PricingMode::Tree
    }

    /// Create Monte Carlo pricing mode.
    pub fn monte_carlo(num_paths: usize) -> Self {
        PricingMode::MonteCarlo {
            num_paths: num_paths.max(100),
            antithetic: true,
        }
    }

    /// Create hybrid pricing mode.
    pub fn hybrid(tree_periods: usize, mc_paths: usize) -> Self {
        PricingMode::Hybrid {
            tree_periods: tree_periods.max(6),
            mc_paths: mc_paths.max(100),
        }
    }
}

/// Configuration for stochastic pricer.
#[derive(Clone)]
pub struct StochasticPricerConfig {
    /// Valuation date
    pub valuation_date: Date,

    /// Discount curve for present value calculations
    pub discount_curve: Arc<DiscountCurve>,

    /// Pricing mode (tree, MC, or hybrid)
    pub pricing_mode: PricingMode,

    /// Scenario tree configuration
    pub tree_config: ScenarioTreeConfig,

    /// Whether to compute risk metrics (EL, UL, ES)
    pub compute_risk_metrics: bool,

    /// Expected Shortfall confidence level (e.g., 0.95 for 95% ES)
    pub es_confidence: f64,

    /// Whether to generate tranche-level cashflows
    pub generate_cashflows: bool,

    /// Random seed for Monte Carlo
    pub seed: u64,
}

impl StochasticPricerConfig {
    /// Create a new pricer configuration.
    pub fn new(
        valuation_date: Date,
        discount_curve: Arc<DiscountCurve>,
        tree_config: ScenarioTreeConfig,
    ) -> Self {
        Self {
            valuation_date,
            discount_curve,
            pricing_mode: PricingMode::default(),
            tree_config,
            compute_risk_metrics: true,
            es_confidence: 0.95,
            generate_cashflows: false,
            seed: 42,
        }
    }

    /// Create RMBS-standard configuration.
    pub fn rmbs_standard(
        valuation_date: Date,
        discount_curve: Arc<DiscountCurve>,
        pool_coupon: f64,
        horizon_years: f64,
    ) -> Self {
        let tree_config = ScenarioTreeConfig::rmbs_standard(horizon_years, pool_coupon);
        Self {
            valuation_date,
            discount_curve,
            pricing_mode: PricingMode::Tree,
            tree_config,
            compute_risk_metrics: true,
            es_confidence: 0.95,
            generate_cashflows: false,
            seed: 42,
        }
    }

    /// Create CLO-standard configuration.
    pub fn clo_standard(
        valuation_date: Date,
        discount_curve: Arc<DiscountCurve>,
        horizon_years: f64,
    ) -> Self {
        let tree_config = ScenarioTreeConfig::clo_standard(horizon_years);
        Self {
            valuation_date,
            discount_curve,
            pricing_mode: PricingMode::Tree,
            tree_config,
            compute_risk_metrics: true,
            es_confidence: 0.95,
            generate_cashflows: false,
            seed: 42,
        }
    }

    /// Set pricing mode.
    pub fn with_pricing_mode(mut self, mode: PricingMode) -> Self {
        self.pricing_mode = mode;
        self
    }

    /// Set whether to compute risk metrics.
    pub fn with_risk_metrics(mut self, compute: bool) -> Self {
        self.compute_risk_metrics = compute;
        self
    }

    /// Set ES confidence level.
    pub fn with_es_confidence(mut self, confidence: f64) -> Self {
        self.es_confidence = confidence.clamp(0.80, 0.9999);
        self
    }

    /// Set whether to generate cashflows.
    pub fn with_cashflows(mut self, generate: bool) -> Self {
        self.generate_cashflows = generate;
        self
    }

    /// Set random seed.
    pub fn with_seed(mut self, seed: u64) -> Self {
        self.seed = seed;
        self.tree_config = self.tree_config.with_seed(seed);
        self
    }

    /// Check if using tree pricing mode.
    pub fn is_tree_mode(&self) -> bool {
        matches!(self.pricing_mode, PricingMode::Tree)
    }

    /// Check if using Monte Carlo pricing mode.
    pub fn is_monte_carlo_mode(&self) -> bool {
        matches!(self.pricing_mode, PricingMode::MonteCarlo { .. })
    }
}

impl std::fmt::Debug for StochasticPricerConfig {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("StochasticPricerConfig")
            .field("valuation_date", &self.valuation_date)
            .field("pricing_mode", &self.pricing_mode)
            .field("compute_risk_metrics", &self.compute_risk_metrics)
            .field("es_confidence", &self.es_confidence)
            .finish()
    }
}

#[cfg(test)]
#[allow(clippy::expect_used, clippy::panic)]
mod tests {
    use super::*;
    use finstack_core::market_data::term_structures::DiscountCurve;
    use finstack_core::math::interp::InterpStyle;
    use time::Month;

    fn test_discount_curve() -> Arc<DiscountCurve> {
        Arc::new(
            DiscountCurve::builder("USD-OIS")
                .base_date(Date::from_calendar_date(2024, Month::January, 15).expect("Valid date"))
                .knots([
                    (0.0, 1.0),
                    (0.5, 0.975),
                    (1.0, 0.95),
                    (2.0, 0.90),
                    (5.0, 0.78),
                ])
                .interp(InterpStyle::LogLinear)
                .build()
                .expect("Valid curve"),
        )
    }

    fn test_date() -> Date {
        Date::from_calendar_date(2024, Month::January, 15).expect("Valid date")
    }

    #[test]
    fn test_pricing_mode_default() {
        let mode = PricingMode::default();
        assert!(matches!(mode, PricingMode::Tree));
    }

    #[test]
    fn test_config_creation() {
        let today = test_date();
        let curve = test_discount_curve();
        let tree_config = ScenarioTreeConfig::new(
            12,
            1.0,
            crate::instruments::fixed_income::structured_credit::pricing::stochastic::tree::BranchingSpec::fixed(
                3,
            ),
        );

        let config = StochasticPricerConfig::new(today, curve, tree_config);

        assert_eq!(config.valuation_date, today);
        assert!(config.is_tree_mode());
        assert!(config.compute_risk_metrics);
    }

    #[test]
    fn test_builder_pattern() {
        let today = test_date();
        let curve = test_discount_curve();
        let tree_config = ScenarioTreeConfig::new(
            12,
            1.0,
            crate::instruments::fixed_income::structured_credit::pricing::stochastic::tree::BranchingSpec::fixed(
                3,
            ),
        );

        let config = StochasticPricerConfig::new(today, curve, tree_config)
            .with_pricing_mode(PricingMode::monte_carlo(10000))
            .with_risk_metrics(true)
            .with_es_confidence(0.99)
            .with_seed(12345);

        assert!(config.is_monte_carlo_mode());
        assert!((config.es_confidence - 0.99).abs() < 1e-10);
        assert_eq!(config.seed, 12345);
    }
}
