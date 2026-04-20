//! Kyle (1985) linear price impact model.
//!
//! A simple model where price impact is proportional to signed order flow.
//! Useful for quick screening and as a calibration target.
//!
//! # References
//!
//! - Kyle, A.S. (1985). "Continuous Auctions and Insider Trading."
//!   *Econometrica*, 53(6). `docs/REFERENCES.md#kyle1985ContinuousAuctions`

use crate::error::{Error, Result};
use serde::{Deserialize, Serialize};

use super::impact::{ExecutionTrajectory, ImpactEstimate, MarketImpactModel, TradeParams};

/// Kyle (1985) price impact model.
///
/// A simple linear model where price impact is proportional to
/// signed order flow:
///
/// ```text
/// delta_price = lambda * order_flow
/// ```
///
/// Lambda can be estimated from the Amihud ratio or regressed from
/// trade-and-quote data.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct KyleLambdaModel {
    /// Price impact per unit of order flow.
    lambda: f64,
}

impl KyleLambdaModel {
    /// Create a new Kyle model with a given lambda.
    ///
    /// # Errors
    ///
    /// Returns `Error::InvalidInput` if `lambda` is negative or non-finite.
    pub fn new(lambda: f64) -> Result<Self> {
        if !lambda.is_finite() || lambda < 0.0 {
            return Err(Error::invalid_input(
                "lambda must be finite and non-negative",
            ));
        }
        Ok(Self { lambda })
    }

    /// Estimate lambda directly from observed volume and return series using
    /// the Amihud-ratio proxy.
    ///
    /// ```text
    /// lambda ~= mean(|r_t| / V_t) * mean(V_t)
    /// ```
    ///
    /// Returns `None` when the inputs are empty, mismatched in length, or
    /// otherwise invalid (zero/non-finite mean volume, ill-defined Amihud ratio).
    pub fn lambda_from_series(volumes: &[f64], returns: &[f64]) -> Option<f64> {
        if volumes.is_empty() || volumes.len() != returns.len() {
            return None;
        }
        let illiq = super::amihud_illiquidity(returns, volumes)?;
        let mean_vol: f64 = volumes.iter().sum::<f64>() / volumes.len() as f64;
        if !mean_vol.is_finite() || mean_vol <= 0.0 {
            return None;
        }
        Self::from_amihud(illiq, mean_vol).ok().map(|m| m.lambda())
    }

    /// Estimate lambda from a liquidity profile using the Amihud proxy.
    ///
    /// ```text
    /// lambda ~= amihud_ratio * avg_daily_volume
    /// ```
    ///
    /// # Errors
    ///
    /// Returns `Error::InvalidInput` if inputs are non-finite or negative.
    pub fn from_amihud(amihud_ratio: f64, avg_daily_volume: f64) -> Result<Self> {
        if !amihud_ratio.is_finite() || amihud_ratio < 0.0 {
            return Err(Error::invalid_input(
                "amihud_ratio must be finite and non-negative",
            ));
        }
        if !avg_daily_volume.is_finite() || avg_daily_volume < 0.0 {
            return Err(Error::invalid_input(
                "avg_daily_volume must be finite and non-negative",
            ));
        }
        Self::new(amihud_ratio * avg_daily_volume)
    }

    /// Get the lambda parameter.
    #[inline]
    pub fn lambda(&self) -> f64 {
        self.lambda
    }
}

impl MarketImpactModel for KyleLambdaModel {
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

        // Kyle model: total price impact = lambda * |Q|
        // Cost = lambda * Q^2 / 2 (integrated impact over linear execution)
        let total_cost = self.lambda * q * q * 0.5;
        let total_cost_abs = total_cost.abs();

        // In the linear model, all impact is "permanent" in the sense
        // that it's a linear function of cumulative order flow.
        // We split heuristically: 60% permanent, 40% temporary.
        let permanent_impact = 0.6 * total_cost_abs;
        let temporary_impact = 0.4 * total_cost_abs;

        let reference_price = params.effective_reference_price();
        let notional = q.abs() * reference_price;
        let cost_bps = if notional > 0.0 {
            total_cost_abs / notional * 10_000.0
        } else {
            0.0
        };

        // Execution risk estimate based on volatility over the horizon
        let execution_risk =
            params.daily_volatility * params.horizon_days.sqrt() * q.abs() * reference_price;

        Ok(ImpactEstimate {
            permanent_impact,
            temporary_impact,
            total_cost: total_cost_abs,
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

        let q = params.quantity;
        let t = params.horizon_days;
        let dt = t / num_buckets as f64;

        // For the Kyle linear model, the optimal trajectory is uniform
        // (constant trading rate) when there's no risk aversion on timing,
        // because the impact is purely a function of total quantity.
        let per_bucket = q / num_buckets as f64;

        let mut quantities = Vec::with_capacity(num_buckets);
        let mut remaining = Vec::with_capacity(num_buckets + 1);
        let mut time_points = Vec::with_capacity(num_buckets + 1);

        remaining.push(q);
        time_points.push(0.0);

        for j in 1..=num_buckets {
            quantities.push(per_bucket);
            let rem = if j == num_buckets {
                0.0
            } else {
                q - per_bucket * j as f64
            };
            remaining.push(rem);
            time_points.push(j as f64 * dt);
        }

        // Expected cost under uniform execution
        let expected_cost = self.lambda * q * q * 0.5;
        let expected_cost_abs = expected_cost.abs();

        // Variance of cost: depends on volatility and remaining inventory
        let sigma = params.daily_volatility * params.effective_reference_price();
        let mut cost_variance = 0.0;
        for j in 0..num_buckets {
            cost_variance += sigma * sigma * remaining[j + 1] * remaining[j + 1] * dt;
        }

        Ok(ExecutionTrajectory {
            quantities,
            remaining,
            expected_cost: expected_cost_abs,
            cost_variance,
            time_points,
        })
    }

    fn model_name(&self) -> &str {
        "Kyle-Lambda"
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::liquidity::types::LiquidityProfile;

    fn test_profile() -> std::result::Result<LiquidityProfile, Box<dyn std::error::Error>> {
        Ok(LiquidityProfile::new(
            "TEST",
            100.0,
            99.5,
            100.5,
            1_000_000.0,
            500.0,
            0.001,
        )?)
    }

    fn test_params(quantity: f64) -> std::result::Result<TradeParams, Box<dyn std::error::Error>> {
        Ok(TradeParams {
            quantity,
            horizon_days: 5.0,
            daily_volatility: 0.02,
            profile: test_profile()?,
            risk_aversion: None,
            reference_price: None,
        })
    }

    #[test]
    fn construction_valid() {
        assert!(KyleLambdaModel::new(0.001).is_ok());
        assert!(KyleLambdaModel::new(0.0).is_ok()); // zero lambda is valid
    }

    #[test]
    fn construction_rejects_negative() {
        assert!(KyleLambdaModel::new(-0.001).is_err());
    }

    #[test]
    fn construction_rejects_nan() {
        assert!(KyleLambdaModel::new(f64::NAN).is_err());
    }

    #[test]
    fn from_amihud_basic() -> std::result::Result<(), Box<dyn std::error::Error>> {
        let model = KyleLambdaModel::from_amihud(1e-9, 1_000_000.0);
        assert!(model.is_ok());
        let m = model?;
        assert!((m.lambda() - 0.001).abs() < 1e-10);
        Ok(())
    }

    #[test]
    fn estimate_cost_nonnegative() -> std::result::Result<(), Box<dyn std::error::Error>> {
        let model = KyleLambdaModel::new(0.001)?;
        let params = test_params(10_000.0)?;
        let est = model.estimate_cost(&params)?;

        assert!(est.total_cost >= 0.0);
        assert!(est.cost_bps >= 0.0);
        Ok(())
    }

    #[test]
    fn estimate_cost_consistent_with_amihud() -> std::result::Result<(), Box<dyn std::error::Error>>
    {
        // If lambda = amihud * ADV, then the per-unit price impact
        // for a trade of size Q is lambda * Q.
        // For Q=1000 shares at lambda=0.001:
        // price_impact = 0.001 * 1000 = 1.0
        // total_cost = 0.001 * 1000^2 / 2 = 500.0
        let model = KyleLambdaModel::new(0.001)?;
        let params = test_params(1_000.0)?;
        let est = model.estimate_cost(&params)?;
        assert!(
            (est.total_cost - 500.0).abs() < 1e-6,
            "expected 500.0, got {}",
            est.total_cost
        );
        Ok(())
    }

    #[test]
    fn trajectory_uniform_execution() -> std::result::Result<(), Box<dyn std::error::Error>> {
        let model = KyleLambdaModel::new(0.001)?;
        let params = test_params(10_000.0)?;
        let traj = model.optimal_trajectory(&params, 5)?;

        assert_eq!(traj.quantities.len(), 5);
        let expected = 10_000.0 / 5.0;
        for q in &traj.quantities {
            assert!(
                (q - expected).abs() < 1e-6,
                "expected uniform {expected}, got {q}"
            );
        }

        let total: f64 = traj.quantities.iter().sum();
        assert!((total - 10_000.0).abs() < 1e-6);
        Ok(())
    }

    #[test]
    fn trajectory_remaining_monotone() -> std::result::Result<(), Box<dyn std::error::Error>> {
        let model = KyleLambdaModel::new(0.001)?;
        let params = test_params(10_000.0)?;
        let traj = model.optimal_trajectory(&params, 10)?;

        for i in 1..traj.remaining.len() {
            assert!(
                traj.remaining[i] <= traj.remaining[i - 1] + 1e-10,
                "remaining should be monotonically decreasing"
            );
        }
        Ok(())
    }

    #[test]
    fn trajectory_rejects_zero_buckets() -> std::result::Result<(), Box<dyn std::error::Error>> {
        let model = KyleLambdaModel::new(0.001)?;
        assert!(model.optimal_trajectory(&test_params(1000.0)?, 0).is_err());
        Ok(())
    }
}
