//! Almgren-Chriss (2001) market impact model.
//!
//! Decomposes market impact into permanent and temporary components
//! and solves for the optimal execution trajectory that minimizes
//! expected cost + risk aversion * cost variance.
//!
//! # References
//!
//! - Almgren, R. & Chriss, N. (2001). "Optimal Execution of Portfolio
//!   Transactions." *Journal of Risk*, 3(2).
//!   `docs/REFERENCES.md#almgrenChriss2001OptimalExecution`

use crate::error::{Error, Result};
use serde::{Deserialize, Serialize};

use super::impact::{ExecutionTrajectory, ImpactEstimate, MarketImpactModel, TradeParams};
use super::types::LiquidityProfile;

/// Almgren-Chriss (2001) market impact model.
///
/// Decomposes market impact into permanent and temporary components:
///
/// **Permanent impact**: proportional to total volume traded.
/// ```text
/// g(v) = gamma * v
/// ```
///
/// **Temporary impact**: order-flow pressure following a power law.
/// ```text
/// h(v) = eta * sign(v) * |v|^delta
/// ```
///
/// where `v` is the trading rate, `gamma` is the permanent impact coefficient,
/// `eta` is the temporary impact coefficient, and `delta` is the power-law
/// exponent (typically 0.5-0.6).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct AlmgrenChrissModel {
    /// Permanent impact coefficient (gamma).
    gamma: f64,

    /// Temporary impact coefficient (eta).
    eta: f64,

    /// Power-law exponent for temporary impact (delta).
    /// Typically 0.5-0.6 for equities.
    delta: f64,
}

impl AlmgrenChrissModel {
    /// Create a new Almgren-Chriss model.
    ///
    /// # Arguments
    ///
    /// * `gamma` - Permanent impact coefficient. Must be non-negative.
    /// * `eta` - Temporary impact coefficient. Must be positive.
    /// * `delta` - Power-law exponent. Must be in (0, 1].
    ///
    /// # Errors
    ///
    /// Returns `Error::InvalidInput` if parameters are out of valid range.
    pub fn new(gamma: f64, eta: f64, delta: f64) -> Result<Self> {
        if !gamma.is_finite() || gamma < 0.0 {
            return Err(Error::invalid_input(
                "gamma must be finite and non-negative",
            ));
        }
        if !eta.is_finite() || eta <= 0.0 {
            return Err(Error::invalid_input("eta must be finite and positive"));
        }
        if !delta.is_finite() || delta <= 0.0 || delta > 1.0 {
            return Err(Error::invalid_input("delta must be in (0, 1]"));
        }

        Ok(Self { gamma, eta, delta })
    }

    /// Estimate parameters from a `LiquidityProfile`.
    ///
    /// Uses the empirical calibration approach:
    /// - `gamma` estimated from a fraction of the spread per unit volume
    /// - `eta` estimated from spread and volume relationship
    /// - `delta` set to a default of 0.5
    ///
    /// # Arguments
    ///
    /// * `profile` - Liquidity profile for the instrument.
    /// * `daily_volatility` - Daily return volatility.
    ///
    /// # Errors
    ///
    /// Returns `Error::InvalidInput` if daily_volatility is non-positive or
    /// if the profile has zero volume.
    pub fn from_profile(profile: &LiquidityProfile, daily_volatility: f64) -> Result<Self> {
        if !daily_volatility.is_finite() || daily_volatility <= 0.0 {
            return Err(Error::invalid_input(
                "daily_volatility must be finite and positive",
            ));
        }
        if profile.avg_daily_volume <= 0.0 {
            return Err(Error::invalid_input(
                "avg_daily_volume must be positive for calibration",
            ));
        }

        // Empirical calibration:
        // gamma ~ spread / (2 * ADV) -- permanent impact per share
        let gamma = profile.relative_spread() / (2.0 * profile.avg_daily_volume);

        // eta ~ daily_volatility * sqrt(mid / ADV)
        // This calibration makes temporary impact scale with volatility
        // and inversely with the square root of turnover
        let eta = daily_volatility * (profile.mid / profile.avg_daily_volume).sqrt();

        let delta = 0.5;

        Self::new(gamma, eta, delta)
    }

}

impl MarketImpactModel for AlmgrenChrissModel {
    fn estimate_cost(&self, params: &TradeParams) -> Result<ImpactEstimate> {
        if !params.quantity.is_finite() {
            return Err(Error::invalid_input("quantity must be finite"));
        }
        if !params.horizon_days.is_finite() || params.horizon_days <= 0.0 {
            return Err(Error::invalid_input(
                "horizon_days must be finite and positive",
            ));
        }
        if !params.daily_volatility.is_finite() || params.daily_volatility <= 0.0 {
            return Err(Error::invalid_input(
                "daily_volatility must be finite and positive",
            ));
        }

        let q = params.quantity;
        let t = params.horizon_days;

        // Trading rate assuming uniform execution
        let rate = q / t;

        // Permanent impact: gamma * Q (price shift)
        let perm_impact = self.gamma * q.abs();
        let perm_cost = perm_impact * q.abs() * 0.5;

        // Temporary impact: eta * |rate|^delta per period
        let temp_impact_per_period = self.eta * rate.abs().powf(self.delta);
        let temp_cost = temp_impact_per_period * q.abs();

        let total_cost = perm_cost + temp_cost;

        // Notional value
        let notional = q.abs() * params.profile.mid;
        let cost_bps = if notional > 0.0 {
            total_cost / notional * 10_000.0
        } else {
            0.0
        };

        // Execution risk: volatility * sqrt(T) * |Q| * mid
        let execution_risk =
            params.daily_volatility * t.sqrt() * q.abs() * params.profile.mid;

        Ok(ImpactEstimate {
            permanent_impact: perm_cost,
            temporary_impact: temp_cost,
            total_cost,
            cost_bps,
            execution_risk,
        })
    }

    fn optimal_trajectory(
        &self,
        params: &TradeParams,
        num_buckets: usize,
    ) -> Result<ExecutionTrajectory> {
        if num_buckets == 0 {
            return Err(Error::invalid_input("num_buckets must be > 0"));
        }
        if !params.quantity.is_finite() {
            return Err(Error::invalid_input("quantity must be finite"));
        }
        if !params.horizon_days.is_finite() || params.horizon_days <= 0.0 {
            return Err(Error::invalid_input(
                "horizon_days must be finite and positive",
            ));
        }
        if !params.daily_volatility.is_finite() || params.daily_volatility <= 0.0 {
            return Err(Error::invalid_input(
                "daily_volatility must be finite and positive",
            ));
        }

        let q = params.quantity;
        let t = params.horizon_days;
        let dt = t / num_buckets as f64;
        let risk_aversion = params.risk_aversion.unwrap_or(1e-6);
        let sigma = params.daily_volatility * params.profile.mid;

        // For linear temporary impact (delta=1), the optimal trajectory
        // has an analytical solution. For general delta, we use the
        // linear solution as a good approximation (exact when delta=1).
        //
        // kappa = sqrt(risk_aversion * sigma^2 / eta)
        // Optimal remaining: x_j = Q * sinh(kappa * (T - t_j)) / sinh(kappa * T)
        let kappa_sq = if self.eta > 0.0 {
            risk_aversion * sigma * sigma / self.eta
        } else {
            0.0
        };
        let kappa = kappa_sq.sqrt();

        let kappa_t = kappa * t;

        let mut remaining = Vec::with_capacity(num_buckets + 1);
        let mut time_points = Vec::with_capacity(num_buckets + 1);
        let mut quantities = Vec::with_capacity(num_buckets);

        // Generate the optimal remaining position at each time point
        remaining.push(q);
        time_points.push(0.0);

        if kappa_t.abs() < 1e-12 {
            // When kappa ~ 0, uniform execution is optimal
            let per_bucket = q / num_buckets as f64;
            for j in 1..=num_buckets {
                let t_j = j as f64 * dt;
                time_points.push(t_j);
                let rem = q - per_bucket * j as f64;
                remaining.push(rem);
                quantities.push(per_bucket);
            }
        } else {
            let sinh_kt = kappa_t.sinh();
            for j in 1..=num_buckets {
                let t_j = j as f64 * dt;
                time_points.push(t_j);
                let rem = if j == num_buckets {
                    0.0 // Ensure exact completion
                } else {
                    q * (kappa * (t - t_j)).sinh() / sinh_kt
                };
                remaining.push(rem);
            }
            for j in 0..num_buckets {
                quantities.push(remaining[j] - remaining[j + 1]);
            }
        }

        // Compute expected cost and cost variance of the trajectory
        let mut expected_cost = 0.0;
        let mut cost_variance = 0.0;

        for j in 0..num_buckets {
            let trade_rate = quantities[j] / dt;

            // Permanent impact cost contribution
            expected_cost += self.gamma * quantities[j].abs() * remaining[j].abs();

            // Temporary impact cost contribution
            expected_cost += self.eta * trade_rate.abs().powf(self.delta) * quantities[j].abs();

            // Variance contribution: sigma^2 * remaining^2 * dt
            cost_variance += sigma * sigma * remaining[j + 1] * remaining[j + 1] * dt;
        }

        Ok(ExecutionTrajectory {
            quantities,
            remaining,
            expected_cost,
            cost_variance,
            time_points,
        })
    }

    fn model_name(&self) -> &str {
        "Almgren-Chriss"
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::liquidity::types::LiquidityProfile;

    fn test_profile() -> LiquidityProfile {
        LiquidityProfile::new("TEST", 100.0, 99.5, 100.5, 1_000_000.0, 500.0, 0.001)
            .expect("valid profile")
    }

    fn test_params(quantity: f64) -> TradeParams {
        TradeParams {
            quantity,
            horizon_days: 5.0,
            daily_volatility: 0.02,
            profile: test_profile(),
            risk_aversion: None,
        }
    }

    #[test]
    fn construction_valid() {
        assert!(AlmgrenChrissModel::new(0.001, 0.01, 0.5).is_ok());
    }

    #[test]
    fn construction_rejects_negative_gamma() {
        assert!(AlmgrenChrissModel::new(-0.001, 0.01, 0.5).is_err());
    }

    #[test]
    fn construction_rejects_zero_eta() {
        assert!(AlmgrenChrissModel::new(0.001, 0.0, 0.5).is_err());
    }

    #[test]
    fn construction_rejects_delta_out_of_range() {
        assert!(AlmgrenChrissModel::new(0.001, 0.01, 0.0).is_err());
        assert!(AlmgrenChrissModel::new(0.001, 0.01, 1.5).is_err());
    }

    #[test]
    fn from_profile_calibrates() {
        let profile = test_profile();
        let model = AlmgrenChrissModel::from_profile(&profile, 0.02);
        assert!(model.is_ok());
    }

    #[test]
    fn estimate_cost_nonnegative() {
        let model = AlmgrenChrissModel::new(0.001, 0.01, 0.5).expect("valid");
        let params = test_params(10_000.0);
        let est = model.estimate_cost(&params).expect("valid");

        assert!(est.total_cost >= 0.0);
        assert!(est.permanent_impact >= 0.0);
        assert!(est.temporary_impact >= 0.0);
        assert!(est.cost_bps >= 0.0);
        assert!(est.execution_risk >= 0.0);
    }

    #[test]
    fn estimate_cost_sell_side() {
        let model = AlmgrenChrissModel::new(0.001, 0.01, 0.5).expect("valid");
        let params = test_params(-10_000.0);
        let est = model.estimate_cost(&params).expect("valid");
        assert!(est.total_cost >= 0.0, "sell-side cost should be non-negative");
    }

    #[test]
    fn estimate_cost_scales_with_quantity() {
        let model = AlmgrenChrissModel::new(0.001, 0.01, 0.5).expect("valid");
        let small = model.estimate_cost(&test_params(1_000.0)).expect("valid");
        let large = model.estimate_cost(&test_params(100_000.0)).expect("valid");
        assert!(
            large.total_cost > small.total_cost,
            "larger trade should cost more"
        );
    }

    #[test]
    fn trajectory_sums_to_quantity() {
        let model = AlmgrenChrissModel::new(0.001, 0.01, 0.5).expect("valid");
        let params = test_params(50_000.0);
        let traj = model.optimal_trajectory(&params, 10).expect("valid");

        assert_eq!(traj.quantities.len(), 10);
        assert_eq!(traj.remaining.len(), 11);
        assert_eq!(traj.time_points.len(), 11);

        let total_traded: f64 = traj.quantities.iter().sum();
        assert!(
            (total_traded - 50_000.0).abs() < 1e-6,
            "trajectory should trade entire quantity, got {total_traded}"
        );

        // Remaining should start at Q and end at 0
        assert!((traj.remaining[0] - 50_000.0).abs() < 1e-10);
        assert!(traj.remaining[10].abs() < 1e-10);
    }

    #[test]
    fn trajectory_zero_risk_aversion_is_uniform() {
        let model = AlmgrenChrissModel::new(0.001, 0.01, 1.0).expect("valid");
        let mut params = test_params(10_000.0);
        params.risk_aversion = Some(0.0);
        let traj = model.optimal_trajectory(&params, 5).expect("valid");

        // With zero risk aversion, kappa = 0 => uniform execution
        let expected_per_bucket = 10_000.0 / 5.0;
        for q in &traj.quantities {
            assert!(
                (q - expected_per_bucket).abs() < 1e-6,
                "expected uniform {expected_per_bucket}, got {q}"
            );
        }
    }

    #[test]
    fn trajectory_rejects_zero_buckets() {
        let model = AlmgrenChrissModel::new(0.001, 0.01, 0.5).expect("valid");
        let params = test_params(10_000.0);
        assert!(model.optimal_trajectory(&params, 0).is_err());
    }

    #[test]
    fn serde_round_trip_impact_estimate() {
        let est = ImpactEstimate {
            permanent_impact: 100.0,
            temporary_impact: 200.0,
            total_cost: 300.0,
            cost_bps: 15.0,
            execution_risk: 500.0,
        };
        let json = serde_json::to_string(&est).expect("serialize");
        let est2: ImpactEstimate = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(est, est2);
    }
}
