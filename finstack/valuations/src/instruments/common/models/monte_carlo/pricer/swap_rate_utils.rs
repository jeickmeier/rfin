//! Shared utilities for swap rate calculation from Hull-White model.
//!
//! Provides reusable functions for computing forward swap rates and bond prices
//! from Hull-White short rate simulations. Used by both swaption and CMS pricing.

use super::super::payoff::swaption::SwapSchedule;
use crate::instruments::common::mc::process::ou::HullWhite1FParams;

/// Hull-White bond price calculation utilities.
///
/// Computes P(t, T) = A(t, T) * exp(-B(t, T) * r(t))
///
/// where:
/// - B(t, T) = (1 - exp(-κ(T-t))) / κ
/// - A(t, T) depends on model parameters
pub struct HullWhiteBondPrice;

impl HullWhiteBondPrice {
    /// Compute B(t, T) factor for bond price.
    ///
    /// B factor represents the sensitivity of bond price to short rate.
    #[allow(non_snake_case)]
    pub fn b_factor(kappa: f64, t: f64, maturity_time: f64) -> f64 {
        if kappa.abs() < 1e-10 {
            // Limit as κ → 0: B(t, T) = T - t
            maturity_time - t
        } else {
            (1.0 - (-kappa * (maturity_time - t)).exp()) / kappa
        }
    }

    /// Compute A(t, T) factor for bond price (simplified for constant θ).
    ///
    /// Full formula with time-dependent θ requires integration. This simplified
    /// version uses the average θ over [t, T] as an approximation.
    ///
    /// # Arguments
    ///
    /// * `params` - Hull-White parameters
    /// * `t` - Current time
    /// * `maturity_time` - Maturity time (T)
    /// * `discount_curve_fn` - Function to get market discount factors
    #[allow(non_snake_case)]
    pub fn a_factor(
        params: &HullWhite1FParams,
        t: f64,
        maturity_time: f64,
        discount_curve_fn: impl Fn(f64) -> f64,
    ) -> f64 {
        let kappa = params.kappa;
        let sigma = params.sigma;
        let B = Self::b_factor(kappa, t, maturity_time);
        let tau = maturity_time - t;

        // Approximate θ as average over [t, T]
        // For piecewise constant, use midpoint
        let theta_mid = params.theta_at_time((t + maturity_time) / 2.0);

        // Market discount factor at T (from discount curve)
        let df_T = discount_curve_fn(maturity_time);
        let df_t = discount_curve_fn(t);

        // Use market forward rate to calibrate
        // Forward rate approximation: f(t,T) ≈ -ln(P_market(T)/P_market(t)) / (T-t)
        let forward_rate = if tau > 1e-10 {
            -(df_T / df_t).ln() / tau
        } else {
            theta_mid
        };

        // Simplified A factor: ensures bond price matches market at t=0
        // More sophisticated: integrate θ(s) over [t, T]
        let term1 = forward_rate * tau;
        let term2 = forward_rate * B;
        let term3 = (sigma * sigma) / (2.0 * kappa * kappa) * (B - tau);
        let term4 = (sigma * sigma) / (4.0 * kappa) * B * B;

        (term1 - term2 + term3 + term4).exp()
    }

    /// Compute bond price P(t, T) from short rate r(t).
    ///
    /// # Arguments
    ///
    /// * `params` - Hull-White parameters
    /// * `r_t` - Current short rate
    /// * `t` - Current time
    /// * `maturity_time` - Maturity time (T)
    /// * `discount_curve_fn` - Function to get market discount factors
    #[allow(non_snake_case)]
    pub fn bond_price(
        params: &HullWhite1FParams,
        r_t: f64,
        t: f64,
        maturity_time: f64,
        discount_curve_fn: impl Fn(f64) -> f64,
    ) -> f64 {
        let B = Self::b_factor(params.kappa, t, maturity_time);
        let A = Self::a_factor(params, t, maturity_time, discount_curve_fn);
        A * (-B * r_t).exp()
    }
}

/// Forward swap rate calculation from Hull-White model.
///
/// Computes S(t) = [P(t, T_0) - P(t, T_N)] / A(t)
///
/// where:
/// - P(t, T_i) are bond prices
/// - A(t) is the annuity (sum of accrual-weighted bond prices)
pub struct ForwardSwapRate;

impl ForwardSwapRate {
    /// Compute forward swap rate at time t from short rate r(t).
    ///
    /// # Arguments
    ///
    /// * `params` - Hull-White parameters
    /// * `r_t` - Current short rate
    /// * `t` - Current time
    /// * `schedule` - Swap schedule
    /// * `discount_curve_fn` - Function to get market discount factors
    pub fn compute(
        params: &HullWhite1FParams,
        r_t: f64,
        t: f64,
        schedule: &SwapSchedule,
        discount_curve_fn: impl Fn(f64) -> f64,
    ) -> f64 {
        // Only compute if t < swap start
        if t >= schedule.end_date {
            return 0.0; // Swap has expired
        }

        // Compute bond prices for swap start and end
        let p_start = if t <= schedule.start_date {
            HullWhiteBondPrice::bond_price(params, r_t, t, schedule.start_date, &discount_curve_fn)
        } else {
            // After swap start, use current time as start
            1.0
        };

        let p_end =
            HullWhiteBondPrice::bond_price(params, r_t, t, schedule.end_date, &discount_curve_fn);

        // Compute annuity: A(t) = Σ τ_i * P(t, T_i)
        let mut annuity = 0.0;
        for (i, &payment_time) in schedule.payment_dates.iter().enumerate() {
            if payment_time > t {
                let p_i = HullWhiteBondPrice::bond_price(
                    params,
                    r_t,
                    t,
                    payment_time,
                    &discount_curve_fn,
                );
                let tau_i = schedule.accrual_fractions[i];
                annuity += tau_i * p_i;
            }
        }

        // Forward swap rate
        if annuity > 1e-10 {
            (p_start - p_end) / annuity
        } else {
            0.0
        }
    }

    /// Compute convexity adjustment for CMS rate (simplified approximation).
    ///
    /// Convexity adjustment accounts for the difference between CMS rate
    /// and forward swap rate due to volatility.
    ///
    /// # Arguments
    ///
    /// * `volatility` - Swap rate volatility
    /// * `tenor` - Time to fixing date
    /// * `swap_tenor` - Tenor of the CMS swap
    ///
    /// # Returns
    ///
    /// Convexity adjustment to add to forward swap rate
    pub fn convexity_adjustment(volatility: f64, tenor: f64, swap_tenor: f64) -> f64 {
        // Simplified convexity adjustment
        // More sophisticated: use full volatility smile and correlation
        0.5 * volatility * volatility * tenor * swap_tenor / (1.0 + 0.03 * swap_tenor)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hw_bond_price_b_factor() {
        let kappa = 0.1;
        let t = 0.0;
        let t_maturity = 1.0;
        let b = HullWhiteBondPrice::b_factor(kappa, t, t_maturity);

        // B(0,1) with κ=0.1 should be approximately (1 - exp(-0.1)) / 0.1 ≈ 0.9516
        let expected = (1.0 - (-0.1_f64).exp()) / 0.1;
        assert!((b - expected).abs() < 1e-10);
    }

    #[test]
    fn test_forward_swap_rate_simple() {
        let params = HullWhite1FParams::new(0.1, 0.01, 0.03);
        let r_t = 0.03;
        let t = 0.0;

        let payment_dates = vec![1.0, 1.25, 1.5, 1.75, 2.0];
        let accruals = vec![0.25, 0.25, 0.25, 0.25, 0.25];
        let schedule = SwapSchedule::new(1.0, 2.0, payment_dates, accruals);

        // Simple discount curve: DF(t) = exp(-0.03 * t)
        let discount_fn = |t: f64| (-0.03 * t).exp();

        let swap_rate = ForwardSwapRate::compute(&params, r_t, t, &schedule, discount_fn);

        // Swap rate should be positive and reasonable
        assert!(swap_rate > 0.0);
        assert!(swap_rate < 1.0);
    }

    #[test]
    fn test_convexity_adjustment() {
        let adj = ForwardSwapRate::convexity_adjustment(0.20, 1.0, 10.0);

        // Should be positive (convexity adjustment increases CMS rate)
        assert!(adj > 0.0);
        // With 20% vol, 1Y tenor, 10Y swap: 0.5 * 0.04 * 10 / 1.3 ≈ 0.154
        assert!(adj < 0.2); // Should be reasonable size
        assert!((adj - 0.15384615384615388).abs() < 1e-6); // Check exact value
    }
}
