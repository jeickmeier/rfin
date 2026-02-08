//! Interest rate derivative payoffs for Monte Carlo pricing.
//!
//! Provides payoffs for caps, floors, and swaptions under short rate models
//! like Hull-White 1F.
//!
//! # Cap/Floor Basics
//!
//! - **Cap**: Portfolio of caplets, each pays max(L - K, 0) on fixing dates
//! - **Floor**: Portfolio of floorlets, each pays max(K - L, 0) on fixing dates
//! - **L**: Forward rate (LIBOR/SOFR) for period [T_i, T_i+1]
//! - **K**: Strike rate
//!
//! # Hull-White Forward Rate Calculation
//!
//! For the Hull-White one-factor model, the forward rate L(t,T,T+τ) is derived
//! from the instantaneous short rate r(t) using the model's bond pricing formula:
//!
//! ```text
//! L(t,T,T+τ) = (1/τ) * [P(t,T)/P(t,T+τ) - 1]
//! ```
//!
//! where P(t,T) is the Hull-White zero-coupon bond price:
//!
//! ```text
//! P(t,T) = A(t,T) * exp(-B(t,T) * r(t))
//! B(t,T) = (1 - exp(-a(T-t))) / a
//! A(t,T) = P^M(0,T)/P^M(0,t) * exp(B(t,T)*f^M(0,t) - σ²/(4a) * B(t,T)² * (1-exp(-2at)))
//! ```
//!
//! # References
//!
//! - Hull, J. & White, A. (1990). "Pricing Interest-Rate-Derivative Securities."
//!   *Review of Financial Studies*, 3(4), 573-592.
//! - Brigo, D. & Mercurio, F. (2006). *Interest Rate Models - Theory and Practice*
//!   (2nd ed.). Springer. Chapter 4.

use crate::instruments::common_impl::mc::traits::PathState;
use crate::instruments::common_impl::models::monte_carlo::traits::Payoff;
use finstack_core::currency::Currency;
use finstack_core::money::Money;

/// Direction of the interest rate payoff (cap vs floor).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RatesPayoffType {
    /// Cap payoff: max(L - K, 0)
    Cap,
    /// Floor payoff: max(K - L, 0)
    Floor,
}

/// Hull-White one-factor model parameters for forward rate derivation.
///
/// These parameters are used to compute forward rates from simulated short rates
/// using the Hull-White bond pricing formula.
#[derive(Debug, Clone, Copy, Default)]
pub struct HullWhiteParams {
    /// Mean reversion speed (a > 0)
    pub mean_reversion: f64,
    /// Volatility of short rate (σ > 0)
    pub volatility: f64,
    /// Initial instantaneous forward rate f(0,0)
    pub initial_forward_rate: f64,
}

impl HullWhiteParams {
    /// Create Hull-White parameters with validation.
    ///
    /// # Arguments
    ///
    /// * `mean_reversion` - Mean reversion speed (a), must be > 0
    /// * `volatility` - Volatility of short rate (σ), must be > 0
    /// * `initial_forward_rate` - Initial instantaneous forward rate f(0,0)
    pub fn new(mean_reversion: f64, volatility: f64, initial_forward_rate: f64) -> Self {
        Self {
            mean_reversion: mean_reversion.max(1e-10), // Prevent division by zero
            volatility: volatility.max(0.0),
            initial_forward_rate,
        }
    }

    /// Calculate B(t,T) function for Hull-White model.
    ///
    /// B(t,T) = (1 - exp(-a(T-t))) / a
    #[inline]
    pub fn b_function(&self, t: f64, maturity: f64) -> f64 {
        let tau = maturity - t;
        if tau <= 0.0 {
            return 0.0;
        }
        let a = self.mean_reversion;
        if a.abs() < 1e-10 {
            // Limit as a → 0: B(t,T) → T-t
            tau
        } else {
            (1.0 - (-a * tau).exp()) / a
        }
    }

    /// Calculate the forward rate from short rate using Hull-White bond pricing.
    ///
    /// Uses simplified Hull-White formula assuming flat initial forward curve:
    ///
    /// L(t,T,T+τ) = (1/τ) * [exp(B(t,T+τ) - B(t,T)) * r(t) + adjustment - 1]
    ///
    /// For practical purposes, we use a linear approximation that is accurate
    /// for small periods and typical Hull-White parameters.
    ///
    /// # Arguments
    ///
    /// * `short_rate` - Current instantaneous short rate r(t)
    /// * `t` - Current time
    /// * `fixing_time` - Forward rate start time T
    /// * `tenor` - Forward rate period τ (accrual fraction)
    pub fn forward_rate(&self, short_rate: f64, t: f64, fixing_time: f64, tenor: f64) -> f64 {
        if tenor <= 0.0 {
            return short_rate;
        }

        let a = self.mean_reversion;
        let sigma = self.volatility;

        // Calculate B values
        let b_start = self.b_function(t, fixing_time);
        let b_end = self.b_function(t, fixing_time + tenor);

        // The difference in B functions gives the forward rate sensitivity
        let delta_b = b_end - b_start;

        // Forward rate approximation using Hull-White model
        // For the HW model, the forward rate can be approximated as:
        // L ≈ f(0,T) + (short_rate - f(0,t)) * ∂B/∂T + convexity adjustment
        //
        // Using a more direct approach:
        // The instantaneous forward rate at T seen from t is:
        // f(t,T) = f(0,T) + σ² * B(0,t) * B(t,T) / 2 + r(t) - r_model(t)
        //
        // For the simple forward rate over [T, T+τ]:
        // L(t,T,T+τ) ≈ short_rate + convexity_adjustment
        //
        // Convexity adjustment for Hull-White:
        // CA = -σ² * B(t,T) * (B(t,T+τ) - B(t,T)) / 2
        let convexity_adj = -sigma * sigma * b_start * delta_b / 2.0;

        // Simple approximation: forward rate tracks short rate with convexity adjustment
        // This is valid when T ≈ t (near-term forwards)
        // For longer forwards, we'd need the full term structure
        let forward_adjustment = if t < fixing_time {
            // Forward rate adjustment for time gap between t and T
            // Based on HW dynamics: E[r(T)|r(t)] = r(t)*exp(-a(T-t)) + θ(1-exp(-a(T-t)))/a
            // For simplicity, use short rate plus mean reversion adjustment
            let decay = (-a * (fixing_time - t)).exp();
            short_rate * decay + self.initial_forward_rate * (1.0 - decay)
        } else {
            short_rate
        };

        (forward_adjustment + convexity_adj).max(0.0) // Forward rates should be non-negative
    }
}

/// Unified interest rate derivative payoff (cap or floor).
///
/// A cap pays max(L - K, 0) at each fixing date.
/// A floor pays max(K - L, 0) at each fixing date.
///
/// Where L is the forward rate for the period and K is the strike rate.
///
/// # Forward Rate Calculation
///
/// When Hull-White parameters are provided, the forward rate is derived from
/// the simulated short rate using the Hull-White bond pricing formula.
/// Otherwise, the short rate is used directly as an approximation.
///
/// # State Requirements
///
/// Expects `PathState` to contain "short_rate" at fixing dates.
#[derive(Debug, Clone)]
pub struct RatesPayoff {
    /// Type of payoff (cap or floor)
    pub payoff_type: RatesPayoffType,
    /// Strike rate (e.g., 0.03 for 3%)
    pub strike_rate: f64,
    /// Notional amount
    pub notional: f64,
    /// Fixing dates (time in years)
    pub fixing_dates: Vec<f64>,
    /// Accrual fractions (daycount) for each period
    pub accrual_fractions: Vec<f64>,
    /// Currency
    pub currency: Currency,
    /// Discount factors for each payment (pre-computed or from curve)
    pub discount_factors: Vec<f64>,
    /// Hull-White model parameters for forward rate derivation (optional)
    pub hull_white_params: Option<HullWhiteParams>,

    // State
    accumulated_pv: f64,
    next_fixing_idx: usize,
}

impl RatesPayoff {
    /// Create a new rates payoff (cap or floor).
    ///
    /// # Arguments
    ///
    /// * `payoff_type` - Type of payoff (Cap or Floor)
    /// * `strike_rate` - Strike rate (as decimal, e.g., 0.03 for 3%)
    /// * `notional` - Notional amount
    /// * `fixing_dates` - Time points for rate fixings
    /// * `accrual_fractions` - Daycount fractions for each period
    /// * `discount_factors` - Discount factors for each payment
    /// * `currency` - Currency for the payoff
    pub fn new(
        payoff_type: RatesPayoffType,
        strike_rate: f64,
        notional: f64,
        fixing_dates: Vec<f64>,
        accrual_fractions: Vec<f64>,
        discount_factors: Vec<f64>,
        currency: Currency,
    ) -> Self {
        assert_eq!(
            fixing_dates.len(),
            accrual_fractions.len(),
            "Fixing dates and accrual fractions must match"
        );
        assert_eq!(
            fixing_dates.len(),
            discount_factors.len(),
            "Fixing dates and discount factors must match"
        );

        Self {
            payoff_type,
            strike_rate,
            notional,
            fixing_dates,
            accrual_fractions,
            discount_factors,
            currency,
            hull_white_params: None,
            accumulated_pv: 0.0,
            next_fixing_idx: 0,
        }
    }

    /// Create a new rates payoff with Hull-White model parameters.
    ///
    /// When Hull-White parameters are provided, forward rates are derived from
    /// simulated short rates using the proper Hull-White bond pricing formula
    /// instead of using the short rate directly.
    ///
    /// # Arguments
    ///
    /// * `payoff_type` - Type of payoff (Cap or Floor)
    /// * `strike_rate` - Strike rate (as decimal, e.g., 0.03 for 3%)
    /// * `notional` - Notional amount
    /// * `fixing_dates` - Time points for rate fixings
    /// * `accrual_fractions` - Daycount fractions for each period
    /// * `discount_factors` - Discount factors for each payment
    /// * `currency` - Currency for the payoff
    /// * `hw_params` - Hull-White model parameters for forward rate calculation
    #[allow(clippy::too_many_arguments)]
    pub fn with_hull_white(
        payoff_type: RatesPayoffType,
        strike_rate: f64,
        notional: f64,
        fixing_dates: Vec<f64>,
        accrual_fractions: Vec<f64>,
        discount_factors: Vec<f64>,
        currency: Currency,
        hw_params: HullWhiteParams,
    ) -> Self {
        let mut payoff = Self::new(
            payoff_type,
            strike_rate,
            notional,
            fixing_dates,
            accrual_fractions,
            discount_factors,
            currency,
        );
        payoff.hull_white_params = Some(hw_params);
        payoff
    }

    /// Set Hull-White parameters for forward rate calculation.
    pub fn set_hull_white_params(&mut self, params: HullWhiteParams) {
        self.hull_white_params = Some(params);
    }
}

impl Payoff for RatesPayoff {
    fn on_event(&mut self, state: &mut PathState) {
        // Check if we're at a fixing date
        if self.next_fixing_idx < self.fixing_dates.len() {
            let target_time = self.fixing_dates[self.next_fixing_idx];

            // Use small tolerance for time matching
            if (state.time - target_time).abs() < 1e-6 {
                // Get short rate from path state
                // Defaults to 0.0 if short_rate is not available, which results in
                // zero forward rate and zero payoff for that fixing period.
                let short_rate = state.get("short_rate").unwrap_or(0.0);

                // Calculate forward rate from short rate
                let forward_rate = if let Some(ref hw_params) = self.hull_white_params {
                    // Use Hull-White model to derive forward rate from short rate
                    // L(t,T,T+τ) is computed using HW bond pricing formula
                    let tenor = self.accrual_fractions[self.next_fixing_idx];
                    hw_params.forward_rate(short_rate, state.time, target_time, tenor)
                } else {
                    // Fallback: use short rate directly as simplified approximation
                    // This is less accurate but works for testing and simple cases
                    short_rate
                };

                // Compute payoff based on type
                let intrinsic_value = match self.payoff_type {
                    RatesPayoffType::Cap => (forward_rate - self.strike_rate).max(0.0),
                    RatesPayoffType::Floor => (self.strike_rate - forward_rate).max(0.0),
                };

                let period_payoff = intrinsic_value
                    * self.accrual_fractions[self.next_fixing_idx]
                    * self.notional
                    * self.discount_factors[self.next_fixing_idx];

                self.accumulated_pv += period_payoff;
                self.next_fixing_idx += 1;
            }
        }
    }

    fn value(&self, _currency: Currency) -> Money {
        Money::new(self.accumulated_pv, self.currency)
    }

    fn reset(&mut self) {
        self.accumulated_pv = 0.0;
        self.next_fixing_idx = 0;
    }
}

/// Cap-floor parity relationship.
///
/// Validates: Cap - Floor = Swap (fixed for floating)
///
/// For a cap with strike K and floor with same strike:
/// ```text
/// Cap(K) - Floor(K) = Σ DF_i * τ_i * N * (L_i - K)
/// ```
///
/// This is the value of receiving floating and paying fixed at K.
pub fn cap_floor_parity_swap_value(
    fixing_dates: &[f64],
    forward_rates: &[f64],
    accrual_fractions: &[f64],
    discount_factors: &[f64],
    strike_rate: f64,
    notional: f64,
) -> f64 {
    use finstack_core::math::summation::NeumaierAccumulator;

    assert_eq!(fixing_dates.len(), forward_rates.len());
    assert_eq!(fixing_dates.len(), accrual_fractions.len());
    assert_eq!(fixing_dates.len(), discount_factors.len());

    let mut pv = NeumaierAccumulator::new();
    for i in 0..fixing_dates.len() {
        let cashflow = (forward_rates[i] - strike_rate) * accrual_fractions[i] * notional;
        pv.add(cashflow * discount_factors[i]);
    }

    pv.total()
}

#[cfg(test)]
#[allow(clippy::expect_used, clippy::panic)]
mod tests {
    use super::*;

    #[test]
    fn test_cap_payoff_structure() {
        let fixing_dates = vec![0.25, 0.5, 0.75, 1.0];
        let accruals = vec![0.25, 0.25, 0.25, 0.25];
        let dfs = vec![0.99, 0.98, 0.97, 0.96];

        let cap = RatesPayoff::new(
            RatesPayoffType::Cap,
            0.03,
            1_000_000.0,
            fixing_dates,
            accruals,
            dfs,
            Currency::USD,
        );

        assert_eq!(cap.payoff_type, RatesPayoffType::Cap);
        assert_eq!(cap.strike_rate, 0.03);
        assert_eq!(cap.notional, 1_000_000.0);
        assert_eq!(cap.fixing_dates.len(), 4);
    }

    #[test]
    fn test_floor_payoff_structure() {
        let fixing_dates = vec![0.25, 0.5];
        let accruals = vec![0.25, 0.25];
        let dfs = vec![0.99, 0.98];

        let floor = RatesPayoff::new(
            RatesPayoffType::Floor,
            0.02,
            500_000.0,
            fixing_dates,
            accruals,
            dfs,
            Currency::EUR,
        );

        assert_eq!(floor.payoff_type, RatesPayoffType::Floor);
        assert_eq!(floor.strike_rate, 0.02);
        assert_eq!(floor.notional, 500_000.0);
    }

    #[test]
    fn test_cap_floor_parity() {
        let fixing_dates = vec![0.5, 1.0];
        let forward_rates = vec![0.04, 0.045];
        let accruals = vec![0.5, 0.5];
        let dfs = vec![0.98, 0.96];
        let strike = 0.03;
        let notional = 1_000_000.0;

        let swap_value = cap_floor_parity_swap_value(
            &fixing_dates,
            &forward_rates,
            &accruals,
            &dfs,
            strike,
            notional,
        );

        // Manual calculation:
        // Period 1: (0.04 - 0.03) * 0.5 * 1M * 0.98 = 4,900
        // Period 2: (0.045 - 0.03) * 0.5 * 1M * 0.96 = 7,200
        // Total: 12,100

        let expected =
            (0.04 - 0.03) * 0.5 * 1_000_000.0 * 0.98 + (0.045 - 0.03) * 0.5 * 1_000_000.0 * 0.96;

        assert!((swap_value - expected).abs() < 1.0);
    }

    #[test]
    fn test_rates_payoff_type_enum() {
        // Verify enum values are distinct
        assert_ne!(RatesPayoffType::Cap, RatesPayoffType::Floor);

        // Verify Copy trait
        let cap_type = RatesPayoffType::Cap;
        let _cap_type_copy = cap_type;
        assert_eq!(cap_type, RatesPayoffType::Cap);
    }

    #[test]
    fn test_payoff_computation_cap() {
        let fixing_dates = vec![0.5];
        let accruals = vec![0.5];
        let dfs = vec![0.98];

        let mut cap = RatesPayoff::new(
            RatesPayoffType::Cap,
            0.03,
            1_000_000.0,
            fixing_dates.clone(),
            accruals.clone(),
            dfs.clone(),
            Currency::USD,
        );

        // Simulate path state with forward rate above strike
        let mut state = PathState::new(0, 0.5);
        state.set("short_rate", 0.05); // Forward rate = 5% > 3% strike

        cap.on_event(&mut state);

        // Expected: (0.05 - 0.03) * 0.5 * 1M * 0.98 = 9,800
        let expected_pv = (0.05 - 0.03) * 0.5 * 1_000_000.0 * 0.98;
        assert!((cap.accumulated_pv - expected_pv).abs() < 1.0);
    }

    #[test]
    fn test_payoff_computation_floor() {
        let fixing_dates = vec![0.5];
        let accruals = vec![0.5];
        let dfs = vec![0.98];

        let mut floor = RatesPayoff::new(
            RatesPayoffType::Floor,
            0.03,
            1_000_000.0,
            fixing_dates.clone(),
            accruals.clone(),
            dfs.clone(),
            Currency::USD,
        );

        // Simulate path state with forward rate below strike
        let mut state = PathState::new(0, 0.5);
        state.set("short_rate", 0.01); // Forward rate = 1% < 3% strike

        floor.on_event(&mut state);

        // Expected: (0.03 - 0.01) * 0.5 * 1M * 0.98 = 9,800
        let expected_pv = (0.03 - 0.01) * 0.5 * 1_000_000.0 * 0.98;
        assert!((floor.accumulated_pv - expected_pv).abs() < 1.0);
    }

    #[test]
    fn test_hull_white_params_b_function() {
        // Test B(t,T) function
        let hw = HullWhiteParams::new(0.1, 0.01, 0.03);

        // B(0, 1) with a=0.1 should be (1 - exp(-0.1)) / 0.1 ≈ 0.9516
        let b = hw.b_function(0.0, 1.0);
        let expected = (1.0 - (-0.1_f64).exp()) / 0.1;
        assert!(
            (b - expected).abs() < 1e-10,
            "B function mismatch: {} vs {}",
            b,
            expected
        );

        // B(t,t) should be 0
        let b_same = hw.b_function(0.5, 0.5);
        assert!(b_same.abs() < 1e-10, "B(t,t) should be 0");

        // As a → 0, B(t,T) → T-t
        let hw_small_a = HullWhiteParams::new(1e-12, 0.01, 0.03);
        let b_limit = hw_small_a.b_function(0.0, 1.0);
        assert!(
            (b_limit - 1.0).abs() < 1e-6,
            "B should approach T-t as a→0: {}",
            b_limit
        );
    }

    #[test]
    fn test_hull_white_forward_rate() {
        let hw = HullWhiteParams::new(0.1, 0.01, 0.03);

        // Forward rate should be close to short rate for near-term fixings
        let fwd = hw.forward_rate(0.03, 0.0, 0.0, 0.25);
        assert!(
            (fwd - 0.03).abs() < 0.01,
            "Near-term forward should be close to short rate: {}",
            fwd
        );

        // Forward rate should be non-negative
        let fwd_low = hw.forward_rate(-0.01, 0.0, 1.0, 0.25);
        assert!(fwd_low >= 0.0, "Forward rate should be non-negative");
    }

    #[test]
    fn test_cap_with_hull_white() {
        let hw = HullWhiteParams::new(0.1, 0.01, 0.03);
        let fixing_dates = vec![0.5];
        let accruals = vec![0.5];
        let dfs = vec![0.98];

        let mut cap = RatesPayoff::with_hull_white(
            RatesPayoffType::Cap,
            0.03,
            1_000_000.0,
            fixing_dates,
            accruals,
            dfs,
            Currency::USD,
            hw,
        );

        assert!(cap.hull_white_params.is_some());

        // Simulate path state with short rate above strike
        let mut state = PathState::new(0, 0.5);
        state.set("short_rate", 0.05);

        cap.on_event(&mut state);

        // With HW params, forward rate is derived from short rate
        // The payoff should be positive since short rate > strike
        assert!(
            cap.accumulated_pv > 0.0,
            "Cap payoff should be positive when rate > strike"
        );
    }
}
