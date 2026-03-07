//! Option pricing formulas for interest rate derivatives.
//!
//! This module provides closed-form pricing formulas for European options under:
//! - **Bachelier (Normal) model**: Used for EUR swaptions (post-2015), negative rates
//! - **Black-76 (Lognormal) model**: Standard for USD/GBP swaptions, caps/floors
//! - **Shifted Black model**: For low/negative rate environments
//!
//! All prices assume a unit annuity (PV01 = 1). To get the actual option price,
//! multiply by the annuity factor: `price = annuity × formula_price`.
//!
//! # Market Conventions
//!
//! | Currency | Model | Reference |
//! |----------|-------|-----------|
//! | USD | Black-76 (lognormal) | Hull Ch. 29 |
//! | EUR | Bachelier (normal) | Post-2015 convention |
//! | GBP | Black-76 (lognormal) | Hull Ch. 29 |
//!
//! # References
//!
//! - Hull, J. C. (2018). *Options, Futures, and Other Derivatives* (10th ed.). Chapter 29.
//! - Brigo, D., & Mercurio, F. (2006). *Interest Rate Models - Theory and Practice*.
//!   Springer. Chapters 1, 6.
//! - Bachelier, L. (1900). "Théorie de la spéculation." Annales Scientifiques de l'École
//!   Normale Supérieure.

use crate::math::{norm_cdf, norm_pdf};

// =============================================================================
// Bachelier (Normal) Model
// =============================================================================

/// Bachelier (normal) call price with unit annuity.
///
/// Computes the price of a call option under the Bachelier model assuming a unit annuity (PV01=1).
///
/// # Formula
///
/// ```text
/// Call = (F - K) × N(d) + σ√T × n(d)
/// where d = (F - K) / (σ√T)
/// ```
///
/// # Arguments
/// * `forward` - Forward rate
/// * `strike` - Strike rate
/// * `sigma_n` - Normal volatility (in rate terms, e.g., 0.005 = 50bp)
/// * `t` - Time to expiry in years
///
/// # Returns
/// Call option price per unit annuity
///
/// # Example
/// ```
/// use finstack_core::math::volatility::bachelier_call;
///
/// let forward = 0.02;  // 2% forward rate
/// let strike = 0.025;  // 2.5% strike
/// let sigma = 0.005;   // 50bp normal vol
/// let t = 1.0;         // 1 year
///
/// let price = bachelier_call(forward, strike, sigma, t);
/// assert!(price >= 0.0);
/// ```
pub fn bachelier_call(forward: f64, strike: f64, sigma_n: f64, t: f64) -> f64 {
    if t <= 0.0 || sigma_n <= 0.0 {
        return (forward - strike).max(0.0);
    }
    let st = sigma_n * t.sqrt();
    let d = (forward - strike) / st;
    (forward - strike) * norm_cdf(d) + st * norm_pdf(d)
}

/// Bachelier (normal) put price with unit annuity.
///
/// Computes the price of a put option under the Bachelier model assuming a unit annuity (PV01=1).
///
/// # Formula
///
/// ```text
/// Put = (K - F) × N(-d) + σ√T × n(d)
/// where d = (F - K) / (σ√T)
/// ```
///
/// Equivalently, by put-call parity: `Put = Call - (F - K)`
///
/// # Arguments
/// * `forward` - Forward rate
/// * `strike` - Strike rate
/// * `sigma_n` - Normal volatility (in rate terms)
/// * `t` - Time to expiry in years
///
/// # Returns
/// Put option price per unit annuity
pub fn bachelier_put(forward: f64, strike: f64, sigma_n: f64, t: f64) -> f64 {
    if t <= 0.0 || sigma_n <= 0.0 {
        return (strike - forward).max(0.0);
    }
    let st = sigma_n * t.sqrt();
    let d = (forward - strike) / st;
    (strike - forward) * norm_cdf(-d) + st * norm_pdf(d)
}

/// Bachelier vega: sensitivity of option price to normal volatility.
///
/// This is the same for both calls and puts under Bachelier.
///
/// # Formula
///
/// ```text
/// Vega = √T × n(d)
/// where d = (F - K) / (σ√T)
/// ```
///
/// # Arguments
/// * `forward` - Forward rate
/// * `strike` - Strike rate
/// * `sigma_n` - Normal volatility
/// * `t` - Time to expiry in years
///
/// # Returns
/// Sensitivity of price to a 1 unit change in normal vol (per unit annuity)
///
/// # Note
/// To get vega per 1bp vol change, multiply result by 0.0001.
pub fn bachelier_vega(forward: f64, strike: f64, sigma_n: f64, t: f64) -> f64 {
    if t <= 0.0 || sigma_n <= 0.0 {
        return 0.0;
    }
    let st = sigma_n * t.sqrt();
    let d = (forward - strike) / st;
    t.sqrt() * norm_pdf(d)
}

/// Bachelier delta: sensitivity of call option price to forward rate.
///
/// # Formula
///
/// ```text
/// Delta_call = N(d)
/// where d = (F - K) / (σ√T)
/// ```
///
/// # Returns
/// Sensitivity of call price to a 1 unit change in forward (per unit annuity)
pub fn bachelier_delta_call(forward: f64, strike: f64, sigma_n: f64, t: f64) -> f64 {
    if t <= 0.0 || sigma_n <= 0.0 {
        return if forward >= strike { 1.0 } else { 0.0 };
    }
    let st = sigma_n * t.sqrt();
    let d = (forward - strike) / st;
    norm_cdf(d)
}

/// Bachelier delta: sensitivity of put option price to forward rate.
///
/// # Formula
///
/// ```text
/// Delta_put = N(d) - 1 = -N(-d)
/// ```
///
/// # Returns
/// Sensitivity of put price to a 1 unit change in forward (per unit annuity)
pub fn bachelier_delta_put(forward: f64, strike: f64, sigma_n: f64, t: f64) -> f64 {
    bachelier_delta_call(forward, strike, sigma_n, t) - 1.0
}

/// Bachelier gamma: second derivative of option price with respect to forward.
///
/// This is the same for both calls and puts under the Bachelier model.
///
/// # Formula
///
/// ```text
/// Gamma = n(d) / (σ√T)
/// where d = (F - K) / (σ√T)
/// ```
///
/// # Arguments
/// * `forward` - Forward rate
/// * `strike` - Strike rate
/// * `sigma_n` - Normal volatility
/// * `t` - Time to expiry in years
///
/// # Returns
/// Second derivative of price w.r.t. forward (per unit annuity)
pub fn bachelier_gamma(forward: f64, strike: f64, sigma_n: f64, t: f64) -> f64 {
    if t <= 0.0 || sigma_n <= 0.0 {
        return 0.0;
    }
    let st = sigma_n * t.sqrt();
    let d = (forward - strike) / st;
    norm_pdf(d) / st
}

// =============================================================================
// Black-76 (Lognormal) Model
// =============================================================================

/// Black-76 (lognormal) call price with unit annuity.
///
/// Computes the price of a call option under the Black model assuming a unit annuity (PV01=1).
///
/// # Formula
///
/// ```text
/// Call = F × N(d₁) - K × N(d₂)
/// where:
///   d₁ = [ln(F/K) + ½σ²T] / (σ√T)
///   d₂ = d₁ - σ√T
/// ```
///
/// # Arguments
/// * `forward` - Forward rate (must be positive)
/// * `strike` - Strike rate (must be positive)
/// * `sigma` - Lognormal volatility (e.g., 0.20 = 20%)
/// * `t` - Time to expiry in years
///
/// # Returns
/// Call option price per unit annuity
///
/// # Example
/// ```
/// use finstack_core::math::volatility::black_call;
///
/// let forward = 0.05;  // 5% forward rate
/// let strike = 0.05;   // ATM strike
/// let sigma = 0.20;    // 20% lognormal vol
/// let t = 1.0;         // 1 year
///
/// let price = black_call(forward, strike, sigma, t);
/// assert!(price > 0.0);
/// ```
pub fn black_call(forward: f64, strike: f64, sigma: f64, t: f64) -> f64 {
    if t <= 0.0 {
        return (forward - strike).max(0.0);
    }
    if sigma <= 0.0 || forward <= 0.0 || strike <= 0.0 {
        return (forward - strike).max(0.0);
    }
    let st = sigma * t.sqrt();
    let d1 = ((forward / strike).ln() + 0.5 * st * st) / st;
    let d2 = d1 - st;
    forward * norm_cdf(d1) - strike * norm_cdf(d2)
}

/// Black-76 (lognormal) put price with unit annuity.
///
/// # Formula
///
/// ```text
/// Put = K × N(-d₂) - F × N(-d₁)
/// ```
///
/// Equivalently, by put-call parity: `Put = Call - (F - K)`
///
/// # Arguments
/// * `forward` - Forward rate (must be positive)
/// * `strike` - Strike rate (must be positive)
/// * `sigma` - Lognormal volatility
/// * `t` - Time to expiry in years
///
/// # Returns
/// Put option price per unit annuity
pub fn black_put(forward: f64, strike: f64, sigma: f64, t: f64) -> f64 {
    if t <= 0.0 {
        return (strike - forward).max(0.0);
    }
    if sigma <= 0.0 || forward <= 0.0 || strike <= 0.0 {
        return (strike - forward).max(0.0);
    }
    let st = sigma * t.sqrt();
    let d1 = ((forward / strike).ln() + 0.5 * st * st) / st;
    let d2 = d1 - st;
    strike * norm_cdf(-d2) - forward * norm_cdf(-d1)
}

/// Black-76 vega: sensitivity of option price to lognormal volatility.
///
/// This is the same for both calls and puts.
///
/// # Formula
///
/// ```text
/// Vega = F × √T × n(d₁)
/// ```
///
/// # Arguments
/// * `forward` - Forward rate
/// * `strike` - Strike rate
/// * `sigma` - Lognormal volatility
/// * `t` - Time to expiry in years
///
/// # Returns
/// Sensitivity of price to a 1 unit change in vol (per unit annuity)
///
/// # Note
/// To get vega per 1% vol change, multiply result by 0.01.
pub fn black_vega(forward: f64, strike: f64, sigma: f64, t: f64) -> f64 {
    if t <= 0.0 || sigma <= 0.0 || forward <= 0.0 || strike <= 0.0 {
        return 0.0;
    }
    let st = sigma * t.sqrt();
    let d1 = ((forward / strike).ln() + 0.5 * st * st) / st;
    forward * t.sqrt() * norm_pdf(d1)
}

/// Black-76 delta: sensitivity of call option price to forward rate.
///
/// # Formula
///
/// ```text
/// Delta_call = N(d₁)
/// ```
///
/// # Returns
/// Sensitivity of call price to a 1 unit change in forward (per unit annuity)
pub fn black_delta_call(forward: f64, strike: f64, sigma: f64, t: f64) -> f64 {
    if t <= 0.0 || sigma <= 0.0 || forward <= 0.0 || strike <= 0.0 {
        return if forward >= strike { 1.0 } else { 0.0 };
    }
    let st = sigma * t.sqrt();
    let d1 = ((forward / strike).ln() + 0.5 * st * st) / st;
    norm_cdf(d1)
}

/// Black-76 delta: sensitivity of put option price to forward rate.
///
/// # Formula
///
/// ```text
/// Delta_put = N(d₁) - 1 = -N(-d₁)
/// ```
///
/// # Returns
/// Sensitivity of put price to a 1 unit change in forward (per unit annuity)
pub fn black_delta_put(forward: f64, strike: f64, sigma: f64, t: f64) -> f64 {
    black_delta_call(forward, strike, sigma, t) - 1.0
}

/// Black-76 gamma: second derivative of option price with respect to forward.
///
/// This is the same for both calls and puts.
///
/// # Formula
///
/// ```text
/// Gamma = n(d₁) / (F × σ × √T)
/// ```
///
/// # Returns
/// Second derivative of price w.r.t. forward (per unit annuity)
pub fn black_gamma(forward: f64, strike: f64, sigma: f64, t: f64) -> f64 {
    if t <= 0.0 || sigma <= 0.0 || forward <= 0.0 || strike <= 0.0 {
        return 0.0;
    }
    let st = sigma * t.sqrt();
    let d1 = ((forward / strike).ln() + 0.5 * st * st) / st;
    norm_pdf(d1) / (forward * st)
}

// =============================================================================
// Shifted Black Model
// =============================================================================

/// Shifted Black call price with unit annuity.
///
/// Handles negative rates by shifting both forward and strike.
///
/// # Arguments
/// * `forward` - Forward rate (can be negative)
/// * `strike` - Strike rate (can be negative)
/// * `sigma` - Lognormal volatility
/// * `t` - Time to expiry in years
/// * `shift` - Shift amount (e.g., 0.03 = 3% shift)
///
/// # Returns
/// Call option price per unit annuity
#[inline]
pub fn black_shifted_call(forward: f64, strike: f64, sigma: f64, t: f64, shift: f64) -> f64 {
    black_call(forward + shift, strike + shift, sigma, t)
}

/// Shifted Black put price with unit annuity.
#[inline]
pub fn black_shifted_put(forward: f64, strike: f64, sigma: f64, t: f64, shift: f64) -> f64 {
    black_put(forward + shift, strike + shift, sigma, t)
}

/// Shifted Black vega with unit annuity.
#[inline]
pub fn black_shifted_vega(forward: f64, strike: f64, sigma: f64, t: f64, shift: f64) -> f64 {
    black_vega(forward + shift, strike + shift, sigma, t)
}

// =============================================================================
// Implied Volatility Initial Guess Approximations
// =============================================================================

/// Brenner-Subrahmanyam approximation for Black-76 implied volatility.
///
/// Provides a near-optimal initial guess for implied volatility solvers,
/// dramatically improving convergence speed compared to arbitrary guesses.
///
/// # Formula
///
/// For ATM options (F ≈ K):
/// ```text
/// σ_approx ≈ √(2π/T) × C / F
/// ```
///
/// For general case:
/// ```text
/// σ_approx ≈ √(2π/T) × |C - P| / (F + K)
/// ```
///
/// where by put-call parity: `|C - P| = |F - K|`
///
/// # Arguments
///
/// * `forward` - Forward price/rate
/// * `strike` - Strike price/rate
/// * `option_price` - Market price of the call option (per unit annuity)
/// * `t` - Time to expiry in years
///
/// # Returns
///
/// Approximate implied volatility (lognormal). Returns 0.2 (20%) as fallback
/// for edge cases.
///
/// # Example
///
/// ```rust
/// use finstack_core::math::volatility::{brenner_subrahmanyam_approx, black_call};
///
/// let forward = 0.05;
/// let strike = 0.05;  // ATM
/// let sigma_actual = 0.25;
/// let t = 1.0;
///
/// let price = black_call(forward, strike, sigma_actual, t);
/// let sigma_approx = brenner_subrahmanyam_approx(forward, strike, price, t);
///
/// // Approximation should be close to actual (within ~10% relative error for ATM)
/// assert!((sigma_approx - sigma_actual).abs() < 0.05);
/// ```
///
/// # References
///
/// - Brenner, M., & Subrahmanyam, M. G. (1988). "A Simple Formula to Compute
///   the Implied Standard Deviation." *Financial Analysts Journal*, 44(5), 80-83.
/// - Corrado, C. J., & Miller, T. W. (1996). "A Note on a Simple, Accurate
///   Formula to Compute Implied Standard Deviations." *Journal of Banking &
///   Finance*, 20(3), 595-603.
///
/// **Note**: This is an ATM-only approximation. The `strike` parameter is
/// accepted for API consistency but does not enter the core formula --
/// for non-ATM options, use a full implied vol solver instead.
#[inline]
pub fn brenner_subrahmanyam_approx(forward: f64, strike: f64, option_price: f64, t: f64) -> f64 {
    const TWO_PI: f64 = 2.0 * std::f64::consts::PI;
    const DEFAULT_VOL: f64 = 0.2;

    // Guard against edge cases
    if t <= 0.0 || option_price <= 0.0 || forward <= 0.0 || strike <= 0.0 {
        return DEFAULT_VOL;
    }

    // ATM case: σ ≈ √(2π/T) × C / F
    // This is the most common use case and gives best results near ATM
    let sqrt_2pi_over_t = (TWO_PI / t).sqrt();
    let sigma = sqrt_2pi_over_t * option_price / forward;

    // Clamp to reasonable bounds [0.01, 5.0] (1% to 500% vol)
    sigma.clamp(0.01, 5.0)
}

/// Manaster-Koehler approximation for Black-76 implied volatility.
///
/// An alternative initial guess that works well for away-from-money options.
///
/// # Formula
///
/// ```text
/// σ_approx = √(2 × |ln(F/K)| / T)
/// ```
///
/// # Arguments
///
/// * `forward` - Forward price/rate
/// * `strike` - Strike price/rate
/// * `t` - Time to expiry in years
///
/// # Returns
///
/// Approximate implied volatility. Returns 0.2 as fallback for edge cases.
///
/// # References
///
/// - Manaster, S., & Koehler, G. (1982). "The Calculation of Implied Variances
///   from the Black-Scholes Model: A Note." *Journal of Finance*, 37(1), 227-230.
#[inline]
pub fn manaster_koehler_approx(forward: f64, strike: f64, t: f64) -> f64 {
    const DEFAULT_VOL: f64 = 0.2;

    // Guard against edge cases
    if t <= 0.0 || forward <= 0.0 || strike <= 0.0 {
        return DEFAULT_VOL;
    }

    let moneyness = (forward / strike).ln().abs();

    // For ATM options, moneyness is 0, return default
    if moneyness < 1e-10 {
        return DEFAULT_VOL;
    }

    let sigma = (2.0 * moneyness / t).sqrt();

    // Clamp to reasonable bounds
    sigma.clamp(0.01, 5.0)
}

/// Combined initial guess for implied volatility solvers.
///
/// Uses Brenner-Subrahmanyam for ATM options and blends with Manaster-Koehler
/// for away-from-money options to provide robust initial guesses across
/// all strike ranges.
///
/// This is the recommended initial guess for Newton-Raphson or Brent solvers
/// when computing implied volatility.
///
/// # Arguments
///
/// * `forward` - Forward price/rate
/// * `strike` - Strike price/rate
/// * `option_price` - Market price of the option
/// * `t` - Time to expiry in years
///
/// # Returns
///
/// Optimal initial volatility guess for the solver.
///
/// # Example
///
/// ```rust
/// use finstack_core::math::volatility::implied_vol_initial_guess;
/// use finstack_core::math::solver::{NewtonSolver, Solver};
///
/// let forward = 0.05;
/// let strike = 0.06;  // OTM
/// let target_price = 0.002;
/// let t = 1.0;
///
/// // Get initial guess
/// let vol_guess = implied_vol_initial_guess(forward, strike, target_price, t);
///
/// // Use as starting point for solver
/// // let solver = NewtonSolver::new();
/// // let implied_vol = solver.solve(|v| black_call(forward, strike, v, t) - target_price, vol_guess);
/// ```
#[inline]
pub fn implied_vol_initial_guess(forward: f64, strike: f64, option_price: f64, t: f64) -> f64 {
    const DEFAULT_VOL: f64 = 0.2;

    if t <= 0.0 || option_price <= 0.0 || forward <= 0.0 || strike <= 0.0 {
        return DEFAULT_VOL;
    }

    // Use Brenner-Subrahmanyam as primary estimate
    let bs_approx = brenner_subrahmanyam_approx(forward, strike, option_price, t);

    // For away-from-money options, blend with Manaster-Koehler
    let moneyness = ((forward / strike).ln()).abs();

    if moneyness > 0.2 {
        // Significantly OTM/ITM: use average of both methods
        let mk_approx = manaster_koehler_approx(forward, strike, t);
        (bs_approx + mk_approx) / 2.0
    } else {
        // Near ATM: Brenner-Subrahmanyam is more accurate
        bs_approx
    }
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
#[allow(clippy::expect_used, clippy::panic)]
mod tests {
    use super::*;

    const EPSILON: f64 = 1e-10;

    #[test]
    fn test_bachelier_put_call_parity() {
        // Put-Call parity: Call - Put = F - K
        let forward = 0.03;
        let strike = 0.025;
        let sigma = 0.005;
        let t = 2.0;

        let call = bachelier_call(forward, strike, sigma, t);
        let put = bachelier_put(forward, strike, sigma, t);

        let parity_diff = call - put - (forward - strike);
        assert!(
            parity_diff.abs() < EPSILON,
            "Bachelier put-call parity violated: diff = {}",
            parity_diff
        );
    }

    #[test]
    fn test_black_put_call_parity() {
        // Put-Call parity: Call - Put = F - K
        let forward = 0.05;
        let strike = 0.04;
        let sigma = 0.20;
        let t = 1.5;

        let call = black_call(forward, strike, sigma, t);
        let put = black_put(forward, strike, sigma, t);

        let parity_diff = call - put - (forward - strike);
        assert!(
            parity_diff.abs() < EPSILON,
            "Black put-call parity violated: diff = {}",
            parity_diff
        );
    }

    #[test]
    fn test_bachelier_atm_symmetry() {
        // ATM: Call = Put when F = K
        let forward = 0.03;
        let strike = 0.03; // ATM
        let sigma = 0.005;
        let t = 1.0;

        let call = bachelier_call(forward, strike, sigma, t);
        let put = bachelier_put(forward, strike, sigma, t);

        assert!(
            (call - put).abs() < EPSILON,
            "Bachelier ATM call != put: {} vs {}",
            call,
            put
        );
    }

    #[test]
    fn test_black_atm_symmetry() {
        // ATM: Call = Put when F = K
        let forward = 0.05;
        let strike = 0.05; // ATM
        let sigma = 0.20;
        let t = 1.0;

        let call = black_call(forward, strike, sigma, t);
        let put = black_put(forward, strike, sigma, t);

        assert!(
            (call - put).abs() < EPSILON,
            "Black ATM call != put: {} vs {}",
            call,
            put
        );
    }

    #[test]
    fn test_bachelier_vega_positive() {
        let forward = 0.03;
        let strike = 0.025;
        let sigma = 0.005;
        let t = 1.0;

        let vega = bachelier_vega(forward, strike, sigma, t);
        assert!(vega > 0.0, "Bachelier vega should be positive");
    }

    #[test]
    fn test_black_vega_positive() {
        let forward = 0.05;
        let strike = 0.045;
        let sigma = 0.20;
        let t = 1.0;

        let vega = black_vega(forward, strike, sigma, t);
        assert!(vega > 0.0, "Black vega should be positive");
    }

    #[test]
    fn test_bachelier_delta_bounds() {
        let forward = 0.03;
        let sigma = 0.005;
        let t = 1.0;

        // ITM call: delta should be close to 1
        let delta_itm = bachelier_delta_call(forward, 0.01, sigma, t);
        assert!(delta_itm > 0.9, "ITM call delta should be close to 1");

        // OTM call: delta should be close to 0
        let delta_otm = bachelier_delta_call(forward, 0.05, sigma, t);
        assert!(delta_otm < 0.1, "OTM call delta should be close to 0");

        // ATM call: delta should be close to 0.5
        let delta_atm = bachelier_delta_call(forward, forward, sigma, t);
        assert!(
            (delta_atm - 0.5).abs() < 0.01,
            "ATM call delta should be ~0.5"
        );
    }

    #[test]
    fn test_black_delta_bounds() {
        let forward = 0.05;
        let sigma = 0.20;
        let t = 1.0;

        // All deltas should be in [0, 1] for calls
        let delta_itm = black_delta_call(forward, 0.02, sigma, t);
        let delta_atm = black_delta_call(forward, forward, sigma, t);
        let delta_otm = black_delta_call(forward, 0.08, sigma, t);

        assert!((0.0..=1.0).contains(&delta_itm));
        assert!((0.0..=1.0).contains(&delta_atm));
        assert!((0.0..=1.0).contains(&delta_otm));

        // Ordering: ITM > ATM > OTM
        assert!(delta_itm > delta_atm);
        assert!(delta_atm > delta_otm);

        // Note: Black-76 ATM delta is NOT 0.5 (unlike Bachelier).
        // At ATM, delta = N(0.5σ√T) > 0.5 due to the drift term in d1.
        // For σ=20%, T=1: delta ≈ N(0.1) ≈ 0.54
        assert!(
            delta_atm > 0.5 && delta_atm < 0.6,
            "Black ATM delta should be ~0.54, got {}",
            delta_atm
        );
    }

    #[test]
    fn test_bachelier_gamma_positive() {
        let forward = 0.03;
        let strike = 0.025;
        let sigma = 0.005;
        let t = 1.0;

        let gamma = bachelier_gamma(forward, strike, sigma, t);
        assert!(gamma > 0.0, "Bachelier gamma should be positive");

        // Gamma should be highest at ATM
        let gamma_atm = bachelier_gamma(forward, forward, sigma, t);
        let gamma_otm = bachelier_gamma(forward, forward + 0.02, sigma, t);
        assert!(
            gamma_atm > gamma_otm,
            "ATM gamma should be higher than OTM gamma"
        );
    }

    #[test]
    fn test_black_gamma_positive() {
        let forward = 0.05;
        let strike = 0.05;
        let sigma = 0.20;
        let t = 1.0;

        let gamma = black_gamma(forward, strike, sigma, t);
        assert!(gamma > 0.0, "Black gamma should be positive");
    }

    #[test]
    fn test_shifted_black_consistency() {
        let forward = 0.02;
        let strike = 0.025;
        let sigma = 0.20;
        let t = 1.0;
        let shift = 0.03;

        // Shifted Black with zero shift should equal regular Black
        let regular = black_call(forward, strike, sigma, t);
        let shifted_zero = black_shifted_call(forward, strike, sigma, t, 0.0);
        assert!(
            (regular - shifted_zero).abs() < EPSILON,
            "Shifted Black with zero shift should equal regular Black"
        );

        // Shifted Black should handle negative forward
        let negative_fwd = -0.01;
        let shifted_price = black_shifted_call(negative_fwd, strike, sigma, t, shift);
        assert!(shifted_price >= 0.0, "Option price should be non-negative");
    }

    #[test]
    fn test_expiry_boundary() {
        // At expiry (t=0), option price = intrinsic value
        let forward = 0.05;
        let strike_itm = 0.03;
        let strike_otm = 0.07;
        let sigma = 0.20;

        // Call at expiry
        assert_eq!(
            bachelier_call(forward, strike_itm, sigma, 0.0),
            forward - strike_itm
        );
        assert_eq!(bachelier_call(forward, strike_otm, sigma, 0.0), 0.0);

        assert_eq!(
            black_call(forward, strike_itm, sigma, 0.0),
            forward - strike_itm
        );
        assert_eq!(black_call(forward, strike_otm, sigma, 0.0), 0.0);

        // Put at expiry
        assert_eq!(bachelier_put(forward, strike_itm, sigma, 0.0), 0.0);
        assert_eq!(
            bachelier_put(forward, strike_otm, sigma, 0.0),
            strike_otm - forward
        );
    }

    #[test]
    fn test_zero_vol_boundary() {
        // At zero vol, option price = intrinsic value
        let forward = 0.05;
        let strike_itm = 0.03;
        let strike_otm = 0.07;
        let t = 1.0;

        assert_eq!(
            bachelier_call(forward, strike_itm, 0.0, t),
            forward - strike_itm
        );
        assert_eq!(bachelier_call(forward, strike_otm, 0.0, t), 0.0);

        assert_eq!(
            black_call(forward, strike_itm, 0.0, t),
            forward - strike_itm
        );
        assert_eq!(black_call(forward, strike_otm, 0.0, t), 0.0);
    }

    // =========================================================================
    // Implied Volatility Approximation Tests
    // =========================================================================

    #[test]
    fn test_brenner_subrahmanyam_atm() {
        // Test ATM case where approximation is most accurate
        let forward = 0.05;
        let strike = 0.05; // ATM
        let sigma_actual = 0.20;
        let t = 1.0;

        let price = black_call(forward, strike, sigma_actual, t);
        let sigma_approx = brenner_subrahmanyam_approx(forward, strike, price, t);

        // For ATM options, approximation should be within 15% relative error
        let rel_error = (sigma_approx - sigma_actual).abs() / sigma_actual;
        assert!(
            rel_error < 0.15,
            "ATM approximation error {:.2}% exceeds 15%",
            rel_error * 100.0
        );
    }

    #[test]
    fn test_brenner_subrahmanyam_various_vols() {
        // Test across range of volatilities
        let forward = 0.05;
        let strike = 0.05; // ATM
        let t = 1.0;

        for sigma_actual in [0.10, 0.20, 0.30, 0.40, 0.50] {
            let price = black_call(forward, strike, sigma_actual, t);
            let sigma_approx = brenner_subrahmanyam_approx(forward, strike, price, t);

            // Should be within 20% relative error for all vols
            let rel_error = (sigma_approx - sigma_actual).abs() / sigma_actual;
            assert!(
                rel_error < 0.20,
                "Approximation for σ={:.0}% has error {:.2}%",
                sigma_actual * 100.0,
                rel_error * 100.0
            );
        }
    }

    #[test]
    fn test_manaster_koehler_otm() {
        // Test OTM case where M-K approximation helps
        let forward = 0.05;
        let strike = 0.07; // OTM
        let t = 1.0;

        let approx = manaster_koehler_approx(forward, strike, t);

        // Should return a reasonable positive volatility
        assert!(approx > 0.0, "Approximation should be positive");
        assert!(approx < 2.0, "Approximation should be reasonable (<200%)");
    }

    #[test]
    fn test_implied_vol_initial_guess_consistency() {
        // Test that combined guess is always in valid range
        let forward = 0.05;
        let t = 1.0;

        for &strike in &[0.03, 0.04, 0.05, 0.06, 0.07] {
            let sigma = 0.25;
            let price = black_call(forward, strike, sigma, t);
            let guess = implied_vol_initial_guess(forward, strike, price, t);

            assert!(
                (0.01..=5.0).contains(&guess),
                "Guess {:.4} for K={:.2} outside valid range",
                guess,
                strike
            );
        }
    }

    #[test]
    fn test_implied_vol_edge_cases() {
        // Edge cases should return default volatility
        assert_eq!(brenner_subrahmanyam_approx(0.05, 0.05, 0.001, 0.0), 0.2); // t = 0
        assert_eq!(brenner_subrahmanyam_approx(0.05, 0.05, 0.0, 1.0), 0.2); // price = 0
        assert_eq!(brenner_subrahmanyam_approx(0.0, 0.05, 0.001, 1.0), 0.2); // forward = 0
        assert_eq!(brenner_subrahmanyam_approx(0.05, 0.0, 0.001, 1.0), 0.2); // strike = 0
    }

    #[test]
    fn test_brenner_subrahmanyam_rates_market() {
        // Test with interest rate market typical values (swaption)
        let forward = 0.03; // 3% forward swap rate
        let strike = 0.03; // ATM
        let sigma_actual = 0.50; // 50% lognormal vol (typical for rates)
        let t = 5.0; // 5Y expiry

        let price = black_call(forward, strike, sigma_actual, t);
        let sigma_approx = brenner_subrahmanyam_approx(forward, strike, price, t);

        // Should be reasonably close
        let rel_error = (sigma_approx - sigma_actual).abs() / sigma_actual;
        assert!(
            rel_error < 0.25,
            "Rates market approximation error {:.2}% exceeds 25%",
            rel_error * 100.0
        );
    }
}
