//! Historical VaR calculation engine.
//!
//! Implements Historical Value-at-Risk using historical simulation methodology.
//! Supports both full revaluation and Taylor approximation (Greeks-based) approaches.

use crate::instruments::common::traits::Instrument;
use crate::metrics::risk::MarketHistory;
use finstack_core::dates::Date;
use finstack_core::market_data::context::MarketContext;
use finstack_core::Result;

/// VaR calculation method.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum VarMethod {
    /// Full revaluation of instrument under each historical scenario.
    ///
    /// Most accurate method - reprices the instrument under each historical
    /// market scenario. Captures all non-linearities and path dependencies.
    FullRevaluation,

    /// Taylor approximation using sensitivities (Greeks).
    ///
    /// Faster method - approximates P&L using pre-computed sensitivities.
    /// Good for linear instruments and large portfolios, but may be
    /// inaccurate for highly non-linear instruments (deep OTM options).
    TaylorApproximation,
}

/// Configuration for VaR calculation.
#[derive(Clone, Debug)]
pub struct VarConfig {
    /// Confidence level (e.g., 0.95 for 95% VaR, 0.99 for 99% VaR)
    pub confidence_level: f64,

    /// VaR calculation method
    pub method: VarMethod,

    /// Number of historical days to use (should match MarketHistory window)
    pub window_days: u32,
}

impl VarConfig {
    /// Create a new VaR configuration with standard settings.
    ///
    /// # Arguments
    ///
    /// * `confidence_level` - Confidence level (e.g., 0.95, 0.99)
    pub fn new(confidence_level: f64) -> Self {
        Self {
            confidence_level,
            method: VarMethod::FullRevaluation,
            window_days: 500,
        }
    }

    /// Standard 95% VaR configuration.
    pub fn var_95() -> Self {
        Self::new(0.95)
    }

    /// Standard 99% VaR configuration.
    pub fn var_99() -> Self {
        Self::new(0.99)
    }

    /// Set the calculation method.
    pub fn with_method(mut self, method: VarMethod) -> Self {
        self.method = method;
        self
    }
}

/// VaR calculation results.
#[derive(Clone, Debug)]
pub struct VarResult {
    /// Value-at-Risk at specified confidence level (always positive)
    pub var: f64,

    /// Expected Shortfall (CVaR) at specified confidence level (always positive)
    ///
    /// Average of all losses exceeding VaR threshold.
    pub expected_shortfall: f64,

    /// Full P&L distribution from historical simulation (sorted, worst first)
    pub pnl_distribution: Vec<f64>,

    /// Number of scenarios used in calculation
    pub num_scenarios: usize,

    /// Confidence level used
    pub confidence_level: f64,
}

impl VarResult {
    /// Create VaR result from P&L distribution.
    ///
    /// # Arguments
    ///
    /// * `pnl_distribution` - Unsorted P&L values from historical simulation
    /// * `confidence_level` - Confidence level for VaR/ES calculation
    pub fn from_distribution(
        mut pnl_distribution: Vec<f64>,
        confidence_level: f64,
    ) -> Result<Self> {
        if pnl_distribution.iter().any(|v| !v.is_finite()) {
            return Err(finstack_core::Error::Validation(
                "VaR P&L distribution contains non-finite values (NaN or inf)".to_string(),
            ));
        }

        // Sort P&L distribution (ascending = worst losses first)
        pnl_distribution.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));

        let num_scenarios = pnl_distribution.len();

        // Handle empty distribution
        if num_scenarios == 0 {
            return Ok(Self {
                var: 0.0,
                expected_shortfall: 0.0,
                pnl_distribution,
                num_scenarios,
                confidence_level,
            });
        }

        // Calculate VaR at confidence level
        let var_index = ((1.0 - confidence_level) * num_scenarios as f64).ceil() as usize;
        let var_index = var_index.saturating_sub(1).min(num_scenarios - 1);
        let var = -pnl_distribution[var_index]; // Negative because losses are negative P&Ls

        // Calculate Expected Shortfall (average of tail losses)
        let tail_size = var_index + 1;
        let expected_shortfall = if tail_size > 0 {
            let sum: f64 = pnl_distribution.iter().take(tail_size).sum();
            -(sum / tail_size as f64) // Negative because losses are negative P&Ls
        } else {
            0.0
        };

        Ok(Self {
            var,
            expected_shortfall,
            pnl_distribution,
            num_scenarios,
            confidence_level,
        })
    }
}

/// Calculate Historical VaR for a single instrument using full revaluation.
///
/// # Arguments
///
/// * `instrument` - The instrument to calculate VaR for
/// * `base_market` - Current market context (base case)
/// * `history` - Historical market scenarios
/// * `as_of` - Valuation date
/// * `config` - VaR configuration
///
/// # Returns
///
/// VaR result including VaR, ES, and full P&L distribution
///
/// # Examples
///
/// ```ignore
/// use finstack_valuations::metrics::risk::{calculate_var, VarConfig, MarketHistory};
///
/// let bond = Bond::fixed(...);
/// let market = MarketContext::new()...;
/// let history = MarketHistory::new(...);
/// let config = VarConfig::var_95();
///
/// let result = calculate_var(&bond, &market, &history, as_of, &config)?;
/// println!("95% VaR: ${:.2}", result.var);
/// println!("95% ES: ${:.2}", result.expected_shortfall);
/// ```
pub fn calculate_var<I>(
    instrument: &I,
    base_market: &MarketContext,
    history: &MarketHistory,
    as_of: Date,
    config: &VarConfig,
) -> Result<VarResult>
where
    I: Instrument + ?Sized,
{
    match config.method {
        VarMethod::FullRevaluation => {
            calculate_var_full_revaluation(instrument, base_market, history, as_of, config)
        }
        VarMethod::TaylorApproximation => {
            // TODO: Implement Taylor approximation
            Err(finstack_core::Error::Validation(
                "Taylor approximation VaR not yet implemented".to_string(),
            ))
        }
    }
}

/// Calculate VaR using full revaluation method.
fn calculate_var_full_revaluation<I>(
    instrument: &I,
    base_market: &MarketContext,
    history: &MarketHistory,
    as_of: Date,
    config: &VarConfig,
) -> Result<VarResult>
where
    I: Instrument + ?Sized,
{
    // Get base case valuation
    let base_value = instrument.value(base_market, as_of)?;

    // Revalue under each historical scenario
    let mut pnls = Vec::with_capacity(history.len());

    for scenario in history.iter() {
        // Apply historical shifts to create scenario market
        let scenario_market = scenario.apply(base_market)?;

        // Revalue instrument under scenario
        let scenario_value = instrument.value(&scenario_market, as_of)?;

        // Calculate P&L (change from base)
        let pnl = scenario_value.amount() - base_value.amount();
        pnls.push(pnl);
    }

    // Calculate VaR and ES from P&L distribution
    VarResult::from_distribution(pnls, config.confidence_level)
}

// =============================================================================
// Portfolio VaR Calculation
// =============================================================================

/// Calculate portfolio VaR with proper date-by-date P&L aggregation.
///
/// This function correctly handles portfolio diversification by:
/// 1. For each historical date, calculating P&L for ALL positions
/// 2. Summing P&Ls across positions for that date
/// 3. Sorting the aggregated portfolio P&L distribution
/// 4. Calculating VaR/ES from the portfolio distribution
///
/// **CRITICAL**: Portfolio VaR ≠ sum of individual VaRs due to diversification.
///
/// # Arguments
///
/// * `instruments` - Vector of instrument references
/// * `base_market` - Base market data
/// * `history` - Historical market scenarios
/// * `as_of` - Valuation date
/// * `config` - VaR configuration
///
/// # Returns
///
/// `VarResult` with portfolio VaR and per-position P&L distributions
///
/// # Examples
///
/// ```ignore
/// let instruments = vec![bond1.as_ref(), bond2.as_ref()];
/// let result = calculate_portfolio_var(
///     &instruments,
///     &market,
///     &history,
///     as_of,
///     &VarConfig::var_95()
/// )?;
/// println!("Portfolio VaR: ${}", result.var);
/// println!("Individual VaR sum would be: ${}", sum_of_individual_vars);
/// println!("Diversification benefit: ${}", sum_of_individual_vars - result.var);
/// ```
pub fn calculate_portfolio_var<I>(
    instruments: &[&I],
    base_market: &MarketContext,
    history: &MarketHistory,
    as_of: Date,
    config: &VarConfig,
) -> Result<VarResult>
where
    I: Instrument + ?Sized,
{
    if instruments.is_empty() {
        return Ok(VarResult {
            var: 0.0,
            expected_shortfall: 0.0,
            pnl_distribution: vec![],
            confidence_level: config.confidence_level,
            num_scenarios: 0,
        });
    }

    // Get base portfolio value (currently unused but kept for future enhancements)
    let _base_portfolio_value: f64 = instruments
        .iter()
        .map(|inst| inst.value(base_market, as_of).map(|m| m.amount()))
        .collect::<Result<Vec<_>>>()?
        .iter()
        .sum();

    // Calculate portfolio P&L for each historical scenario
    // Key: aggregate P&Ls date-by-date across all positions
    let mut portfolio_pnls = Vec::with_capacity(history.len());

    for scenario in history.iter() {
        // Apply scenario to get shifted market
        let shifted_market = scenario.apply(base_market)?;

        // Calculate P&L for each instrument under this scenario
        let scenario_pnls: Vec<f64> = instruments
            .iter()
            .map(|inst| {
                let shifted_value = inst.value(&shifted_market, as_of)?.amount();
                let base_value = inst.value(base_market, as_of)?.amount();
                Ok(shifted_value - base_value)
            })
            .collect::<Result<Vec<_>>>()?;

        // Sum P&Ls across all positions for this date
        let total_portfolio_pnl: f64 = scenario_pnls.iter().sum();
        portfolio_pnls.push(total_portfolio_pnl);
    }

    // Calculate VaR from aggregated portfolio P&L distribution
    VarResult::from_distribution(portfolio_pnls, config.confidence_level)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::instruments::common::traits::Instrument;
    use crate::instruments::Bond;
    use crate::metrics::risk::{MarketHistory, MarketScenario, RiskFactorShift, RiskFactorType};
    use finstack_core::dates::DayCount;
    use finstack_core::market_data::context::MarketContext;
    use finstack_core::market_data::term_structures::discount_curve::DiscountCurve;
    use finstack_core::money::Money;
    use finstack_core::types::{Currency, CurveId};
    use std::sync::Arc;
    use time::macros::date;

    #[test]
    fn test_var_config_creation() {
        let config = VarConfig::var_95();
        assert_eq!(config.confidence_level, 0.95);
        assert_eq!(config.method, VarMethod::FullRevaluation);

        let config = VarConfig::var_99().with_method(VarMethod::TaylorApproximation);
        assert_eq!(config.confidence_level, 0.99);
        assert_eq!(config.method, VarMethod::TaylorApproximation);
    }

    #[test]
    fn test_var_result_from_distribution() {
        // Create synthetic P&L distribution with known values
        let pnls = vec![
            100.0,  // gain
            50.0,   // gain
            0.0,    // no change
            -25.0,  // small loss
            -50.0,  // medium loss
            -100.0, // large loss
            -150.0, // very large loss
            -200.0, // worst loss
        ];

        let result = VarResult::from_distribution(pnls, 0.95).expect("pnl distribution is finite");

        // With 8 scenarios and 95% confidence:
        // Tail size = ceil((1-0.95) * 8) = ceil(0.4) = 1
        // So VaR should be the worst loss = 200
        assert_eq!(result.var, 200.0);
        assert_eq!(result.num_scenarios, 8);

        // ES should be average of tail (just the worst loss in this case)
        assert_eq!(result.expected_shortfall, 200.0);
    }

    #[test]
    fn test_var_result_rejects_nan() {
        let pnls = vec![10.0, f64::NAN, -5.0];
        let err = VarResult::from_distribution(pnls, 0.95).expect_err("should reject NaNs");
        match err {
            finstack_core::error::Error::Validation(msg) => {
                assert!(
                    msg.contains("non-finite"),
                    "error message should mention non-finite values"
                );
            }
            other => panic!("unexpected error variant: {other:?}"),
        }
    }

    #[test]
    fn test_var_calculation_simple_bond() -> Result<()> {
        let as_of = date!(2024 - 01 - 01);

        // Create a simple bond
        let bond = Bond::fixed(
            "TEST-BOND",
            Money::new(100_000.0, Currency::USD),
            0.05,
            as_of,
            date!(2029 - 01 - 01),
            "USD-OIS",
        );

        // Create base market
        let base_curve = DiscountCurve::builder("USD-OIS")
            .base_date(as_of)
            .day_count(DayCount::Act365F)
            .knots(vec![(0.0, 1.0), (5.0, 0.85), (10.0, 0.70)])
            .build()?;

        let base_market = MarketContext::new().insert_discount(base_curve);

        // Create historical scenarios with rate shifts
        let scenarios = vec![
            // Rates up 50bp -> bond value down
            MarketScenario::new(
                date!(2023 - 12 - 31),
                vec![RiskFactorShift {
                    factor: RiskFactorType::DiscountRate {
                        curve_id: CurveId::from("USD-OIS"),
                        tenor_years: 5.0,
                    },
                    shift: 0.0050, // +50bp
                }],
            ),
            // Rates down 30bp -> bond value up
            MarketScenario::new(
                date!(2023 - 12 - 30),
                vec![RiskFactorShift {
                    factor: RiskFactorType::DiscountRate {
                        curve_id: CurveId::from("USD-OIS"),
                        tenor_years: 5.0,
                    },
                    shift: -0.0030, // -30bp
                }],
            ),
            // Small rates up 10bp
            MarketScenario::new(
                date!(2023 - 12 - 29),
                vec![RiskFactorShift {
                    factor: RiskFactorType::DiscountRate {
                        curve_id: CurveId::from("USD-OIS"),
                        tenor_years: 5.0,
                    },
                    shift: 0.0010, // +10bp
                }],
            ),
        ];

        let history = MarketHistory::new(as_of, 3, scenarios);
        let config = VarConfig::var_95();

        // Calculate VaR
        let result = calculate_var(&bond, &base_market, &history, as_of, &config)?;

        // Verify results
        assert_eq!(result.num_scenarios, 3);
        assert!(result.var > 0.0, "VaR should be positive");
        assert!(
            result.expected_shortfall >= result.var,
            "ES should be >= VaR"
        );

        // P&L distribution should have 3 values
        assert_eq!(result.pnl_distribution.len(), 3);

        // Distribution should be sorted (worst first)
        for i in 1..result.pnl_distribution.len() {
            assert!(
                result.pnl_distribution[i] >= result.pnl_distribution[i - 1],
                "P&L distribution should be sorted"
            );
        }

        Ok(())
    }

    #[test]
    fn test_var_empty_history() -> Result<()> {
        let as_of = date!(2024 - 01 - 01);

        let bond = Bond::fixed(
            "TEST-BOND",
            Money::new(100_000.0, Currency::USD),
            0.05,
            as_of,
            date!(2029 - 01 - 01),
            "USD-OIS",
        );

        let base_curve = DiscountCurve::builder("USD-OIS")
            .base_date(as_of)
            .day_count(DayCount::Act365F)
            .knots(vec![(0.0, 1.0), (5.0, 0.85)])
            .build()?;

        let base_market = MarketContext::new().insert_discount(base_curve);

        // Empty history
        let history = MarketHistory::new(as_of, 0, vec![]);
        let config = VarConfig::var_95();

        let result = calculate_var(&bond, &base_market, &history, as_of, &config)?;

        assert_eq!(result.num_scenarios, 0);
        assert_eq!(result.pnl_distribution.len(), 0);

        Ok(())
    }

    #[test]
    fn test_portfolio_var_with_diversification() -> Result<()> {
        let as_of = date!(2024 - 01 - 01);

        // Create two bonds with different maturities
        // Bond 1: 5-year (more rate-sensitive)
        let bond1 = Bond::fixed(
            "BOND-5Y",
            Money::new(100_000.0, Currency::USD),
            0.05,
            as_of,
            date!(2029 - 01 - 01),
            "USD-OIS",
        );

        // Bond 2: 2-year (less rate-sensitive)
        let bond2 = Bond::fixed(
            "BOND-2Y",
            Money::new(100_000.0, Currency::USD),
            0.05,
            as_of,
            date!(2026 - 01 - 01),
            "USD-OIS",
        );

        // Create market
        let curve = DiscountCurve::builder("USD-OIS")
            .base_date(as_of)
            .day_count(DayCount::Act365F)
            .knots(vec![(0.0, 1.0), (5.0, 0.85), (10.0, 0.70)])
            .build()?;

        let market = Arc::new(MarketContext::new().insert_discount(curve));

        // Create historical scenarios with varying rate shifts
        // Some scenarios will cause offsetting P&Ls between the two bonds
        let scenarios = vec![
            // Scenario 1: Rates up (both bonds lose value)
            MarketScenario::new(
                date!(2023 - 12 - 31),
                vec![RiskFactorShift {
                    factor: RiskFactorType::DiscountRate {
                        curve_id: CurveId::from("USD-OIS"),
                        tenor_years: 5.0,
                    },
                    shift: 0.0100, // +100bp
                }],
            ),
            // Scenario 2: Rates down (both bonds gain value)
            MarketScenario::new(
                date!(2023 - 12 - 30),
                vec![RiskFactorShift {
                    factor: RiskFactorType::DiscountRate {
                        curve_id: CurveId::from("USD-OIS"),
                        tenor_years: 5.0,
                    },
                    shift: -0.0075, // -75bp
                }],
            ),
            // Scenario 3: Small rate increase
            MarketScenario::new(
                date!(2023 - 12 - 29),
                vec![RiskFactorShift {
                    factor: RiskFactorType::DiscountRate {
                        curve_id: CurveId::from("USD-OIS"),
                        tenor_years: 5.0,
                    },
                    shift: 0.0025, // +25bp
                }],
            ),
            // Scenario 4: Small rate decrease
            MarketScenario::new(
                date!(2023 - 12 - 28),
                vec![RiskFactorShift {
                    factor: RiskFactorType::DiscountRate {
                        curve_id: CurveId::from("USD-OIS"),
                        tenor_years: 5.0,
                    },
                    shift: -0.0050, // -50bp
                }],
            ),
        ];

        let history = MarketHistory::new(as_of, 4, scenarios);
        let config = VarConfig::var_95();

        // Calculate individual VaRs
        let var1 = calculate_var(&bond1, market.as_ref(), &history, as_of, &config)?;
        let var2 = calculate_var(&bond2, market.as_ref(), &history, as_of, &config)?;
        let sum_individual_vars = var1.var.abs() + var2.var.abs();

        // Calculate portfolio VaR
        let instruments: Vec<&dyn Instrument> = vec![&bond1, &bond2];
        let portfolio_var =
            calculate_portfolio_var(&instruments, market.as_ref(), &history, as_of, &config)?;

        // Verify portfolio VaR <= sum of individual VaRs
        // With only a few scenarios and both bonds having similar rate sensitivity,
        // we might not see diversification benefit in this simple test
        assert!(
            portfolio_var.var.abs() <= sum_individual_vars + 0.01, // Allow small numerical tolerance
            "Portfolio VaR ({}) should be <= sum of individual VaRs ({})",
            portfolio_var.var.abs(),
            sum_individual_vars
        );

        // Calculate diversification benefit (may be zero or small with limited scenarios)
        let diversification_benefit = sum_individual_vars - portfolio_var.var.abs();
        assert!(
            diversification_benefit >= -0.01, // Allow small numerical t olerance
            "Diversification benefit should be non-negative, got {}",
            diversification_benefit
        );

        println!("Individual VaR 1: ${:.2}", var1.var);
        println!("Individual VaR 2: ${:.2}", var2.var);
        println!("Sum of individual VaRs: ${:.2}", sum_individual_vars);
        println!("Portfolio VaR: ${:.2}", portfolio_var.var.abs());
        println!("Diversification benefit: ${:.2}", diversification_benefit);

        Ok(())
    }
}
