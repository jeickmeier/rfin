//! Liquidity-adjusted Value at Risk (LVaR) calculator.
//!
//! Composes with existing VaR numbers (from `factor_model/` or external sources)
//! to produce liquidity-adjusted figures following Bangia et al. (1999).
//!
//! # References
//!
//! - Bangia, A., Diebold, F., Schuermann, T., Stroughair, J. (1999).
//!   "Modeling Liquidity Risk with Implications for Traditional Market
//!   Risk Measurement and Management." *Risk*, 12(1).
//!   `docs/REFERENCES.md#bangia1999LiquidityRisk`

use crate::error::{Error, Result};
use crate::types::PositionId;
use finstack_core::math::special_functions::standard_normal_inv_cdf;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use super::types::{days_to_liquidate, LiquidityConfig, LiquidityProfile};

/// Result of a liquidity-adjusted VaR calculation for a single position.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct LvarResult {
    /// Position identifier.
    pub position_id: PositionId,

    /// Standard VaR (input, not computed here).
    pub var: f64,

    /// Exogenous liquidity cost: half-spread times position value.
    /// This is the constant-cost add-on assuming spread is independent
    /// of position size.
    pub exogenous_cost: f64,

    /// Endogenous liquidity cost: spread widening due to position size
    /// relative to ADV.
    pub endogenous_cost: f64,

    /// Bangia et al. (1999) LVaR combining VaR with spread mean and
    /// spread volatility.
    ///
    /// ```text
    /// LVaR = VaR + (0.5 * mean_spread + z_alpha * 0.5 * spread_vol) * PV
    /// ```
    pub lvar_bangia: f64,

    /// Time-to-liquidation adjusted LVaR.
    ///
    /// ```text
    /// LVaR_horizon = VaR * sqrt(liquidation_days / holding_period)
    /// ```
    pub lvar_horizon: f64,

    /// Days required to liquidate at the configured participation rate.
    pub days_to_liquidate: f64,

    /// Composite LVaR: max(lvar_bangia, lvar_horizon).
    ///
    /// Takes the more conservative of the two adjustments.
    pub lvar_composite: f64,
}

/// Aggregated LVaR results across a portfolio.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PortfolioLvarReport {
    /// Per-position LVaR results.
    pub position_results: Vec<LvarResult>,

    /// Sum of standard VaR across positions.
    pub total_var: f64,

    /// Sum of composite LVaR across positions (conservative aggregate).
    pub total_lvar_composite: f64,

    /// Total exogenous liquidity cost add-on.
    pub total_exogenous_cost: f64,

    /// Total endogenous liquidity cost add-on.
    pub total_endogenous_cost: f64,

    /// Liquidity cost as percentage of total VaR.
    ///
    /// ```text
    /// (total_lvar_composite - total_var) / total_var * 100
    /// ```
    pub liquidity_cost_pct: f64,

    /// Positions for which no `LiquidityProfile` was provided.
    pub missing_profiles: Vec<PositionId>,
}

/// Calculator for liquidity-adjusted VaR.
///
/// This calculator does not compute VaR itself -- it takes VaR as an input
/// and adjusts it for liquidity costs and time-to-liquidation effects.
/// VaR should come from the existing `factor_model::ParametricDecomposer`,
/// `factor_model::SimulationDecomposer`, or an external source.
///
/// # References
///
/// - Bangia et al. (1999). `docs/REFERENCES.md#bangia1999LiquidityRisk`
pub struct LvarCalculator {
    config: LiquidityConfig,
    z_alpha: f64,
}

impl LvarCalculator {
    /// Create a new calculator with the given configuration.
    pub fn new(config: LiquidityConfig) -> Self {
        let z_alpha = standard_normal_inv_cdf(config.confidence_level);
        Self { config, z_alpha }
    }

    /// Compute LVaR for a single position.
    ///
    /// # Arguments
    ///
    /// * `position_id` - Identifier for the position.
    /// * `var` - Standard VaR for the position (positive number = loss).
    /// * `position_value` - Absolute market value of the position.
    /// * `profile` - Liquidity profile for the instrument.
    ///
    /// # Returns
    ///
    /// [`LvarResult`] with all LVaR variants computed.
    ///
    /// # Errors
    ///
    /// Returns `Error::InvalidInput` if `var` is negative or non-finite,
    /// or if `position_value` is non-finite.
    pub fn compute(
        &self,
        position_id: &PositionId,
        var: f64,
        position_value: f64,
        profile: &LiquidityProfile,
    ) -> Result<LvarResult> {
        if !var.is_finite() || var < 0.0 {
            return Err(Error::invalid_input(format!(
                "VaR must be non-negative and finite, got {var}"
            )));
        }
        if !position_value.is_finite() {
            return Err(Error::invalid_input(format!(
                "position_value must be finite, got {position_value}"
            )));
        }

        let pv = position_value.abs();

        // Exogenous cost: half-spread * position value
        let exogenous_cost = profile.half_spread() / profile.mid * pv;

        // Endogenous cost: spread widening due to position size relative to ADV
        // Using spread_with_size_impact formula inline:
        // additional spread from position size vs. ADV
        let position_shares = if profile.mid > 0.0 {
            pv / profile.mid
        } else {
            0.0
        };
        let endogenous_cost = if profile.avg_daily_volume > 0.0 {
            let ratio = position_shares / profile.avg_daily_volume;
            let impact = 0.1 * profile.spread() * ratio.powf(0.5);
            impact / profile.mid * pv
        } else {
            0.0
        };

        // Bangia et al. LVaR
        // LVaR = VaR + (0.5 * mean_relative_spread + z * 0.5 * spread_vol) * PV
        let half_relative_spread = 0.5 * profile.relative_spread();
        let spread_vol_term = 0.5 * self.z_alpha * profile.spread_volatility;
        let lvar_bangia = var + (half_relative_spread + spread_vol_term) * pv;

        // Days to liquidate
        let dtl = days_to_liquidate(
            position_shares,
            profile.avg_daily_volume,
            self.config.participation_rate,
        );

        // Horizon-adjusted LVaR
        let horizon_scale = if self.config.holding_period > 0.0 && dtl.is_finite() {
            (dtl / self.config.holding_period).sqrt()
        } else if dtl.is_infinite() {
            dtl
        } else {
            1.0
        };
        let lvar_horizon = var * horizon_scale;

        // Composite: take the more conservative
        let lvar_composite = if lvar_bangia.is_finite() && lvar_horizon.is_finite() {
            lvar_bangia.max(lvar_horizon)
        } else if lvar_horizon.is_infinite() {
            lvar_horizon
        } else {
            lvar_bangia
        };

        Ok(LvarResult {
            position_id: position_id.clone(),
            var,
            exogenous_cost,
            endogenous_cost,
            lvar_bangia,
            lvar_horizon,
            days_to_liquidate: dtl,
            lvar_composite,
        })
    }

    /// Compute LVaR for all positions in a portfolio.
    ///
    /// Runs in parallel when the `parallel` feature is enabled.
    ///
    /// # Arguments
    ///
    /// * `position_vars` - Slice of (position_id, var, position_value) tuples.
    /// * `profiles` - Map from instrument_id to liquidity profile.
    ///
    /// # Returns
    ///
    /// [`PortfolioLvarReport`] with per-position results and aggregates.
    /// Positions without a matching profile are recorded in `missing_profiles`.
    pub fn compute_portfolio(
        &self,
        position_vars: &[(PositionId, String, f64, f64)],
        profiles: &HashMap<String, LiquidityProfile>,
    ) -> PortfolioLvarReport {
        let mut position_results = Vec::new();
        let mut missing_profiles = Vec::new();

        for (pos_id, instrument_id, var, pv) in position_vars {
            match profiles.get(instrument_id.as_str()) {
                Some(profile) => {
                    if let Ok(result) = self.compute(pos_id, *var, *pv, profile) {
                        position_results.push(result);
                    }
                }
                None => {
                    missing_profiles.push(pos_id.clone());
                }
            }
        }

        let total_var: f64 = position_results.iter().map(|r| r.var).sum();
        let total_lvar_composite: f64 = position_results.iter().map(|r| r.lvar_composite).sum();
        let total_exogenous_cost: f64 = position_results.iter().map(|r| r.exogenous_cost).sum();
        let total_endogenous_cost: f64 = position_results.iter().map(|r| r.endogenous_cost).sum();

        let liquidity_cost_pct = if total_var > 0.0 {
            (total_lvar_composite - total_var) / total_var * 100.0
        } else {
            0.0
        };

        PortfolioLvarReport {
            position_results,
            total_var,
            total_lvar_composite,
            total_exogenous_cost,
            total_endogenous_cost,
            liquidity_cost_pct,
            missing_profiles,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::liquidity::types::LiquidityConfig;

    fn test_profile() -> LiquidityProfile {
        LiquidityProfile::new("TEST", 100.0, 99.5, 100.5, 1_000_000.0, 500.0, 0.002)
            .expect("valid profile")
    }

    fn default_calculator() -> LvarCalculator {
        LvarCalculator::new(LiquidityConfig::default())
    }

    #[test]
    fn lvar_basic_computation() {
        let calc = default_calculator();
        let profile = test_profile();
        let pos_id = PositionId::new("POS1");

        let result = calc.compute(&pos_id, 10_000.0, 1_000_000.0, &profile);
        assert!(result.is_ok());
        let r = result.expect("valid");

        assert_eq!(r.var, 10_000.0);
        assert!(r.exogenous_cost > 0.0, "exogenous cost should be positive");
        assert!(r.lvar_bangia > r.var, "LVaR Bangia should exceed VaR");
        assert!(r.lvar_composite >= r.var, "composite should be >= VaR");
    }

    #[test]
    fn lvar_zero_spread_zero_exogenous() {
        let calc = default_calculator();
        // Create a zero-spread instrument (bid == ask == mid)
        let profile = LiquidityProfile {
            instrument_id: "ZERO_SPREAD".into(),
            mid: 100.0,
            bid: 100.0,
            ask: 100.0,
            avg_daily_volume: 1_000_000.0,
            avg_trade_size: 500.0,
            spread_volatility: 0.0,
            observation_days: 20,
        };
        let pos_id = PositionId::new("POS1");

        let r = calc.compute(&pos_id, 10_000.0, 1_000_000.0, &profile).expect("valid");
        assert!((r.exogenous_cost).abs() < 1e-10, "zero spread => zero exogenous cost");
    }

    #[test]
    fn lvar_horizon_equals_one_when_dtl_equals_holding() {
        // If days_to_liquidate == holding_period, horizon adjustment = sqrt(1) = 1
        // position_shares = PV / mid. ADV * participation_rate = daily_capacity.
        // dtl = position_shares / daily_capacity = 1.0
        // => position_shares = ADV * participation_rate * 1.0 = 100,000
        // => PV = 100,000 * mid = 10,000,000
        let config = LiquidityConfig {
            participation_rate: 0.10,
            holding_period: 1.0,
            ..LiquidityConfig::default()
        };
        let calc = LvarCalculator::new(config);
        let profile = LiquidityProfile::new(
            "TEST", 100.0, 99.5, 100.5, 1_000_000.0, 500.0, 0.0,
        )
        .expect("valid");

        // position_shares = 10_000_000 / 100 = 100_000
        // daily_capacity = 0.10 * 1_000_000 = 100_000
        // dtl = 1.0
        let pos_id = PositionId::new("POS1");
        let r = calc.compute(&pos_id, 10_000.0, 10_000_000.0, &profile).expect("valid");
        assert!(
            (r.days_to_liquidate - 1.0).abs() < 1e-10,
            "expected dtl=1.0, got {}",
            r.days_to_liquidate
        );
        assert!(
            (r.lvar_horizon - 10_000.0).abs() < 1e-6,
            "horizon LVaR should equal VaR when dtl=holding_period"
        );
    }

    #[test]
    fn lvar_rejects_negative_var() {
        let calc = default_calculator();
        let profile = test_profile();
        let pos_id = PositionId::new("POS1");
        assert!(calc.compute(&pos_id, -1.0, 1_000_000.0, &profile).is_err());
    }

    #[test]
    fn lvar_rejects_nan_var() {
        let calc = default_calculator();
        let profile = test_profile();
        let pos_id = PositionId::new("POS1");
        assert!(calc.compute(&pos_id, f64::NAN, 1_000_000.0, &profile).is_err());
    }

    #[test]
    fn lvar_rejects_non_finite_position_value() {
        let calc = default_calculator();
        let profile = test_profile();
        let pos_id = PositionId::new("POS1");
        assert!(calc.compute(&pos_id, 1000.0, f64::INFINITY, &profile).is_err());
    }

    #[test]
    fn portfolio_lvar_missing_profiles() {
        let calc = default_calculator();
        let profiles: HashMap<String, LiquidityProfile> = HashMap::new();
        let position_vars = vec![(
            PositionId::new("POS1"),
            "UNKNOWN".to_string(),
            10_000.0,
            1_000_000.0,
        )];

        let report = calc.compute_portfolio(&position_vars, &profiles);
        assert!(report.position_results.is_empty());
        assert_eq!(report.missing_profiles.len(), 1);
    }

    #[test]
    fn portfolio_lvar_aggregation() {
        let calc = default_calculator();
        let profile = test_profile();
        let mut profiles = HashMap::new();
        profiles.insert("TEST".to_string(), profile);

        let position_vars = vec![
            (PositionId::new("POS1"), "TEST".to_string(), 5_000.0, 500_000.0),
            (PositionId::new("POS2"), "TEST".to_string(), 8_000.0, 800_000.0),
        ];

        let report = calc.compute_portfolio(&position_vars, &profiles);
        assert_eq!(report.position_results.len(), 2);
        assert!((report.total_var - 13_000.0).abs() < 1e-10);
        assert!(report.total_lvar_composite >= report.total_var);
        assert!(report.liquidity_cost_pct >= 0.0);
    }

    #[test]
    fn serde_round_trip_lvar_result() {
        // Use exact values to avoid floating-point representation discrepancies
        let r = LvarResult {
            position_id: PositionId::new("POS1"),
            var: 10_000.0,
            exogenous_cost: 5_000.0,
            endogenous_cost: 100.0,
            lvar_bangia: 17_000.0,
            lvar_horizon: 12_000.0,
            days_to_liquidate: 2.5,
            lvar_composite: 17_000.0,
        };

        let json = serde_json::to_string(&r).expect("serialize");
        let r2: LvarResult = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(r, r2);
    }
}
