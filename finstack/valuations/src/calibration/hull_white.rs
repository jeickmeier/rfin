//! Hull-White one-factor model calibration to European swaptions.
//!
//! Calibrates the two Hull-White parameters (mean reversion κ and short rate
//! volatility σ) by minimising squared swaption price errors using the
//! Levenberg-Marquardt algorithm.
//!
//! # Mathematical Foundation
//!
//! The Hull-White one-factor model specifies the short rate dynamics:
//!
//! ```text
//! dr(t) = [θ(t) − κ r(t)] dt + σ dW(t)
//!
//! where:
//!   κ = mean reversion speed
//!   σ = short rate volatility
//!   θ(t) = time-dependent drift chosen to match the initial term structure
//! ```
//!
//! # Swaption Pricing
//!
//! European swaptions are priced analytically using the Jamshidian (1989)
//! decomposition, which expresses a coupon bond option as a portfolio of
//! zero-coupon bond options under the HW1F model.
//!
//! The zero-coupon bond option volatility is:
//!
//! ```text
//! σ_P(t, T, S) = B(T,S) × σ × √((1 − e^{−2κt}) / (2κ))
//!
//! where B(T,S) = (1/κ)(1 − e^{−κ(S−T)})
//! ```
//!
//! # References
//!
//! - Hull, J. & White, A. (1990). "Pricing Interest-Rate-Derivative Securities."
//!   *Review of Financial Studies*, 3(4), 573-592.
//! - Jamshidian, F. (1989). "An Exact Bond Option Formula."
//!   *Journal of Finance*, 44(1), 205-209.
//! - Brigo, D. & Mercurio, F. (2006). *Interest Rate Models — Theory and Practice*.
//!   Springer Finance (2nd ed.), Chapter 3.

use finstack_core::math::solver::{BrentSolver, Solver};
use finstack_core::math::solver_multi::LevenbergMarquardtSolver;
use finstack_core::math::special_functions::norm_cdf;
use std::collections::BTreeMap;

use crate::calibration::CalibrationReport;

/// Hull-White one-factor model parameters.
///
/// # Examples
///
/// ```rust
/// use finstack_valuations::calibration::hull_white::HullWhiteParams;
///
/// let params = HullWhiteParams::new(0.05, 0.01).unwrap();
/// assert!(params.kappa > 0.0);
/// ```
#[derive(Debug, Clone, Copy, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct HullWhiteParams {
    /// Mean reversion speed (κ > 0).
    pub kappa: f64,
    /// Short rate volatility (σ > 0).
    pub sigma: f64,
}

impl HullWhiteParams {
    /// Construct validated Hull-White parameters.
    ///
    /// # Errors
    ///
    /// Returns an error if `kappa <= 0` or `sigma <= 0`.
    pub fn new(kappa: f64, sigma: f64) -> finstack_core::Result<Self> {
        if kappa <= 0.0 || !kappa.is_finite() {
            return Err(finstack_core::Error::Validation(format!(
                "Hull-White kappa (mean reversion) must be positive, got {kappa}"
            )));
        }
        if sigma <= 0.0 || !sigma.is_finite() {
            return Err(finstack_core::Error::Validation(format!(
                "Hull-White sigma (short rate volatility) must be positive, got {sigma}"
            )));
        }
        Ok(Self { kappa, sigma })
    }

    /// B function: B(t₁, t₂) = (1 − e^{−κ(t₂−t₁)}) / κ
    ///
    /// For small κ, uses the Taylor expansion B ≈ (t₂ − t₁) to avoid
    /// division by near-zero.
    #[must_use]
    pub fn b_function(&self, t1: f64, t2: f64) -> f64 {
        hw_b(self.kappa, t1, t2)
    }

    /// Zero-coupon bond option volatility under HW1F.
    ///
    /// ```text
    /// σ_P(t, T, S) = B(T,S) × σ × √((1 − e^{−2κ(T−t)}) / (2κ))
    /// ```
    ///
    /// # Arguments
    ///
    /// * `t` - Current time
    /// * `big_t` - Option expiry time (T)
    /// * `s` - Bond maturity time (S > T)
    #[must_use]
    pub fn bond_option_vol(&self, t: f64, big_t: f64, s: f64) -> f64 {
        hw_bond_vol(self.kappa, self.sigma, t, big_t, s)
    }
}

/// Market quote for a European swaption used in HW1F calibration.
///
/// Represents an ATM (or off-ATM) European swaption with its market volatility.
#[derive(Debug, Clone, Copy, serde::Serialize, serde::Deserialize)]
pub struct SwaptionQuote {
    /// Swaption expiry in years (T₀).
    pub expiry: f64,
    /// Underlying swap tenor in years (e.g. 5.0 for a 5Y swap).
    pub tenor: f64,
    /// Market-quoted volatility.
    pub volatility: f64,
    /// `true` for normal (Bachelier) vol, `false` for lognormal (Black-76) vol.
    pub is_normal_vol: bool,
}

/// Number of coupon payments per year for the underlying swap in HW1F calibration.
///
/// USD swaps are semi-annual (2), EUR swaps are annual (1).
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize, Default)]
pub enum SwapFrequency {
    /// 1 payment per year (EUR, GBP standard).
    Annual,
    /// 2 payments per year (USD standard).
    #[default]
    SemiAnnual,
    /// 4 payments per year.
    Quarterly,
}

impl SwapFrequency {
    fn periods_per_year(self) -> usize {
        match self {
            Self::Annual => 1,
            Self::SemiAnnual => 2,
            Self::Quarterly => 4,
        }
    }
}

/// Convenience wrapper that uses [`SwapFrequency::SemiAnnual`] (USD standard).
///
/// See [`calibrate_hull_white_to_swaptions_with_frequency`] for full documentation.
pub fn calibrate_hull_white_to_swaptions(
    df: &dyn Fn(f64) -> f64,
    quotes: &[SwaptionQuote],
) -> finstack_core::Result<(HullWhiteParams, CalibrationReport)> {
    calibrate_hull_white_to_swaptions_with_frequency_and_initial_guess(
        df,
        quotes,
        SwapFrequency::default(),
        None,
    )
}

/// Calibrate Hull-White 1-factor parameters to European swaption market data.
///
/// Fits κ (mean reversion) and σ (short rate volatility) by minimising
/// squared differences between model and market swaption prices.
///
/// # Arguments
///
/// * `df` - Discount factor function: `df(t)` returns P(0, t). Must satisfy `df(0) ≈ 1`.
/// * `quotes` - Swaption market data
/// * `frequency` - Coupon frequency of the underlying swap (e.g., semi-annual for USD,
///   annual for EUR). This materially affects the annuity factor and forward swap rate.
///
/// # Returns
///
/// Calibrated [`HullWhiteParams`] and a [`CalibrationReport`] with residual diagnostics.
///
/// # Algorithm
///
/// 1. For each swaption quote, compute the market price from the quoted vol.
/// 2. Model prices are computed analytically via the Jamshidian (1989) decomposition.
/// 3. The Levenberg-Marquardt solver minimises the sum of squared price errors.
/// 4. Uses the unconstrained parameterisation: `(ln κ, ln σ)`.
///
/// # Errors
///
/// Returns an error if:
/// - Fewer than 2 quotes are provided (2 free parameters)
/// - Calibration fails to converge
/// - Discount function returns invalid values
///
/// # Examples
///
/// ```rust,no_run
/// use finstack_valuations::calibration::hull_white::{
///     calibrate_hull_white_to_swaptions_with_frequency, SwaptionQuote, SwapFrequency,
/// };
///
/// let quotes = vec![
///     SwaptionQuote { expiry: 1.0, tenor: 5.0, volatility: 0.005, is_normal_vol: true },
///     SwaptionQuote { expiry: 5.0, tenor: 5.0, volatility: 0.006, is_normal_vol: true },
///     SwaptionQuote { expiry: 10.0, tenor: 5.0, volatility: 0.005, is_normal_vol: true },
/// ];
///
/// // Flat 3% discount curve, semi-annual USD convention
/// let df = |t: f64| (-0.03 * t).exp();
/// let (params, report) = calibrate_hull_white_to_swaptions_with_frequency(
///     &df, &quotes, SwapFrequency::SemiAnnual,
/// ).unwrap();
/// assert!(report.success);
/// ```
pub fn calibrate_hull_white_to_swaptions_with_frequency(
    df: &dyn Fn(f64) -> f64,
    quotes: &[SwaptionQuote],
    frequency: SwapFrequency,
) -> finstack_core::Result<(HullWhiteParams, CalibrationReport)> {
    calibrate_hull_white_to_swaptions_with_frequency_and_initial_guess(df, quotes, frequency, None)
}

/// Variant of Hull-White calibration that accepts optional initial guesses.
pub fn calibrate_hull_white_to_swaptions_with_frequency_and_initial_guess(
    df: &dyn Fn(f64) -> f64,
    quotes: &[SwaptionQuote],
    frequency: SwapFrequency,
    initial_guess: Option<HullWhiteParams>,
) -> finstack_core::Result<(HullWhiteParams, CalibrationReport)> {
    if quotes.len() < 2 {
        return Err(finstack_core::Error::Validation(format!(
            "Need at least 2 swaption quotes for HW1F calibration (2 free parameters), got {}",
            quotes.len()
        )));
    }
    for (i, q) in quotes.iter().enumerate() {
        if q.expiry <= 0.0 || q.tenor <= 0.0 || q.volatility <= 0.0 {
            return Err(finstack_core::Error::Validation(format!(
                "Invalid swaption quote at index {i}: expiry={}, tenor={}, vol={}",
                q.expiry, q.tenor, q.volatility
            )));
        }
    }

    let n_quotes = quotes.len();
    let ppy = frequency.periods_per_year();

    let mut market_prices = Vec::with_capacity(n_quotes);
    let mut annuities = Vec::with_capacity(n_quotes);
    let mut fwd_swap_rates = Vec::with_capacity(n_quotes);

    for q in quotes {
        let (annuity, fwd_rate) = compute_swap_annuity_and_rate(df, q.expiry, q.tenor, ppy);
        let market_price = compute_swaption_market_price(
            annuity,
            fwd_rate,
            q.expiry,
            q.volatility,
            q.is_normal_vol,
        );
        market_prices.push(market_price);
        annuities.push(annuity);
        fwd_swap_rates.push(fwd_rate);
    }

    let kappa_init: f64 = initial_guess.map(|p| p.kappa).unwrap_or(0.05);
    let sigma_init: f64 = initial_guess.map(|p| p.sigma).unwrap_or(0.01);
    let x0 = [kappa_init.ln(), sigma_init.ln()];

    let residuals = |x: &[f64], resid: &mut [f64]| {
        let kappa = x[0].exp();
        let sigma = x[1].exp();

        for (idx, q) in quotes.iter().enumerate() {
            let model_price = hw1f_swaption_price(
                kappa,
                sigma,
                df,
                q.expiry,
                q.tenor,
                fwd_swap_rates[idx],
                ppy,
            );
            if model_price.is_finite() {
                resid[idx] = model_price - market_prices[idx];
            } else {
                resid[idx] = 1.0;
            }
        }
    };

    let solver = LevenbergMarquardtSolver::new()
        .with_tolerance(1e-12)
        .with_max_iterations(300);

    let solution = solver.solve_system_with_dim_stats(residuals, &x0, n_quotes)?;

    let kappa = solution.params[0].exp();
    let sigma = solution.params[1].exp();

    let mut residual_map = BTreeMap::new();
    for (idx, q) in quotes.iter().enumerate() {
        let model_price = hw1f_swaption_price(
            kappa,
            sigma,
            df,
            q.expiry,
            q.tenor,
            fwd_swap_rates[idx],
            ppy,
        );
        let resid = model_price - market_prices[idx];
        let label = format!("{}Yx{}Y", q.expiry, q.tenor);
        residual_map.insert(label, resid);
    }

    let report = CalibrationReport::for_type_with_tolerance(
        "hull_white_1f",
        residual_map,
        solution.stats.iterations,
        1e-6,
    )
    .with_model_version("Hull-White 1F (Jamshidian decomposition)")
    .with_metadata("kappa", format!("{kappa:.6}"))
    .with_metadata("sigma", format!("{sigma:.6}"))
    .with_metadata("initial_kappa", format!("{kappa_init:.6}"))
    .with_metadata("initial_sigma", format!("{sigma_init:.6}"))
    .with_metadata("swap_frequency", format!("{frequency:?}"));

    let params = HullWhiteParams::new(kappa, sigma)?;

    Ok((params, report))
}

// =============================================================================
// Internal helpers
// =============================================================================

/// B(t₁, t₂) = (1 − e^{−κ(t₂−t₁)}) / κ
fn hw_b(kappa: f64, t1: f64, t2: f64) -> f64 {
    let tau = t2 - t1;
    if kappa.abs() < 1e-10 {
        tau
    } else {
        (1.0 - (-kappa * tau).exp()) / kappa
    }
}

/// Zero-coupon bond option volatility:
/// σ_P(t, T, S) = B(T,S) × σ × √((1 − e^{−2κ(T−t)}) / (2κ))
fn hw_bond_vol(kappa: f64, sigma: f64, t: f64, big_t: f64, s: f64) -> f64 {
    let b = hw_b(kappa, big_t, s);
    let var_factor = if kappa.abs() < 1e-10 {
        big_t - t
    } else {
        (1.0 - (-2.0 * kappa * (big_t - t)).exp()) / (2.0 * kappa)
    };
    b * sigma * var_factor.max(0.0).sqrt()
}

/// Compute ln A(t, T) for the HW1F affine bond price model.
///
/// ln A(t,T) = ln(P(0,T)/P(0,t)) + B(t,T) f(0,t) − (σ²/4κ)(1−e^{−2κt}) B(t,T)²
fn hw_ln_a(kappa: f64, sigma: f64, t: f64, big_t: f64, df: &dyn Fn(f64) -> f64) -> f64 {
    let p0t = df(t);
    let p0_big_t = df(big_t);
    let b = hw_b(kappa, t, big_t);

    // Instantaneous forward rate: f(0,t) ≈ −d/dt ln P(0,t)
    let h = (t * 1e-3).clamp(1e-6, 1e-3);
    let f0t = if t > h {
        -(df(t + h).ln() - df(t - h).ln()) / (2.0 * h)
    } else {
        // Near t = 0: use forward difference
        -(df(h).ln()) / h
    };

    let var_term = if kappa.abs() < 1e-10 {
        sigma * sigma * t * b * b / 2.0
    } else {
        sigma * sigma / (4.0 * kappa) * (1.0 - (-2.0 * kappa * t).exp()) * b * b
    };

    (p0_big_t / p0t).ln() + b * f0t - var_term
}

/// Compute annuity and forward swap rate for a swap starting at `t0`
/// with given `tenor` and `periods_per_year` coupon payments.
fn compute_swap_annuity_and_rate(
    df: &dyn Fn(f64) -> f64,
    t0: f64,
    tenor: f64,
    periods_per_year: usize,
) -> (f64, f64) {
    let n_periods = (tenor * periods_per_year as f64).round().max(1.0) as usize;
    let dt = tenor / n_periods as f64;

    let mut annuity = 0.0;
    for i in 1..=n_periods {
        let t_i = t0 + i as f64 * dt;
        annuity += dt * df(t_i);
    }

    let t_n = t0 + tenor;
    let fwd_rate = if annuity > 1e-15 {
        (df(t0) - df(t_n)) / annuity
    } else {
        0.03 // fallback
    };

    (annuity, fwd_rate)
}

/// Compute the market swaption price from the quoted volatility.
fn compute_swaption_market_price(
    annuity: f64,
    fwd_rate: f64,
    expiry: f64,
    vol: f64,
    is_normal: bool,
) -> f64 {
    if is_normal {
        // Bachelier: ATM payer price ≈ annuity × σ_n × √T × √(2/π) ≈ annuity × bachelier_call
        annuity * finstack_core::math::volatility::bachelier_call(fwd_rate, fwd_rate, vol, expiry)
    } else {
        // Black-76: annuity × black_call(F, F, σ, T)
        annuity * finstack_core::math::volatility::black_call(fwd_rate, fwd_rate, vol, expiry)
    }
}

/// Price a European payer swaption under HW1F using Jamshidian decomposition.
///
/// The Jamshidian decomposition expresses a swaption as a portfolio of
/// zero-coupon bond options. The key steps are:
///
/// 1. Find the critical short rate r* where the swap value equals par.
/// 2. Each leg becomes a put on a zero-coupon bond with strike K_i = P_HW(r*, T₀, T_i).
/// 3. Sum the individual zero-coupon bond put prices.
fn hw1f_swaption_price(
    kappa: f64,
    sigma: f64,
    df: &dyn Fn(f64) -> f64,
    t0: f64,
    tenor: f64,
    swap_rate: f64,
    periods_per_year: usize,
) -> f64 {
    let n_periods = (tenor * periods_per_year as f64).round().max(1.0) as usize;
    let dt = tenor / n_periods as f64;

    // Payment dates and cashflows
    let mut payment_times = Vec::with_capacity(n_periods);
    let mut cashflows = Vec::with_capacity(n_periods);
    for i in 1..=n_periods {
        let t_i = t0 + i as f64 * dt;
        payment_times.push(t_i);
        let cf = if i < n_periods {
            swap_rate * dt
        } else {
            1.0 + swap_rate * dt
        };
        cashflows.push(cf);
    }

    // Pre-compute B and ln A for each payment date
    let b_vals: Vec<f64> = payment_times
        .iter()
        .map(|&t_i| hw_b(kappa, t0, t_i))
        .collect();
    let ln_a_vals: Vec<f64> = payment_times
        .iter()
        .map(|&t_i| hw_ln_a(kappa, sigma, t0, t_i, df))
        .collect();

    // Find r* such that Σ c_i × A_i × exp(−B_i × r*) = 1
    // g(r) = Σ c_i exp(ln_A_i − B_i r) − 1
    // g'(r) = −Σ c_i B_i exp(ln_A_i − B_i r)
    let g = |r: f64| -> f64 {
        let mut sum = 0.0;
        for i in 0..n_periods {
            sum += cashflows[i] * (ln_a_vals[i] - b_vals[i] * r).exp();
        }
        sum - 1.0
    };

    let g_prime = |r: f64| -> f64 {
        let mut sum = 0.0;
        for i in 0..n_periods {
            sum -= cashflows[i] * b_vals[i] * (ln_a_vals[i] - b_vals[i] * r).exp();
        }
        sum
    };

    // Initial guess: the instantaneous forward rate at t0
    let h = (t0 * 1e-3).clamp(1e-6, 1e-3);
    let f0t0 = if t0 > h {
        -(df(t0 + h).ln() - df(t0 - h).ln()) / (2.0 * h)
    } else {
        -(df(h).ln()) / h
    };

    // Newton iterations to find r*
    let mut r_star = f0t0;
    let mut newton_converged = false;
    for _ in 0..50 {
        let gv = g(r_star);
        let gp = g_prime(r_star);
        if gp.abs() < 1e-15 {
            break;
        }
        let step = gv / gp;
        r_star -= step;
        if step.abs() < 1e-12 {
            newton_converged = true;
            break;
        }
    }

    // Brent fallback if Newton didn't converge
    if !newton_converged {
        tracing::warn!(
            "HW1F r* Newton solver did not converge (kappa={kappa:.4}, sigma={sigma:.4}), \
             falling back to Brent"
        );
        let bracket_lo = f0t0 - 0.05;
        let bracket_hi = f0t0 + 0.05;
        let brent = BrentSolver::new()
            .tolerance(1e-12)
            .bracket_bounds(bracket_lo, bracket_hi);
        match brent.solve(g, f0t0) {
            Ok(r) => r_star = r,
            Err(_) => {
                // Last resort: keep Newton's best guess
                tracing::warn!("HW1F r* Brent fallback also failed; using Newton's best estimate");
            }
        }
    }

    // Compute strike prices K_i = A_i × exp(−B_i × r*)
    let k_strikes: Vec<f64> = (0..n_periods)
        .map(|i| (ln_a_vals[i] - b_vals[i] * r_star).exp())
        .collect();

    // Sum zero-coupon bond put prices (payer swaption = portfolio of bond puts)
    // ZBO_put(0, T₀, T_i, K_i) = K_i P(0,T₀) N(−d₂) − P(0,T_i) N(−d₁)
    let p0_t0 = df(t0);
    let mut swaption_price = 0.0;

    for i in 0..n_periods {
        let t_i = payment_times[i];
        let p0_ti = df(t_i);
        let sigma_p = hw_bond_vol(kappa, sigma, 0.0, t0, t_i);

        if sigma_p < 1e-15 {
            // Degenerate: intrinsic value
            let put_intrinsic = (k_strikes[i] * p0_t0 - p0_ti).max(0.0);
            swaption_price += cashflows[i] * put_intrinsic;
            continue;
        }

        let d1 = ((p0_ti / (k_strikes[i] * p0_t0)).ln() + 0.5 * sigma_p * sigma_p) / sigma_p;
        let d2 = d1 - sigma_p;

        let put_price = k_strikes[i] * p0_t0 * norm_cdf(-d2) - p0_ti * norm_cdf(-d1);
        swaption_price += cashflows[i] * put_price.max(0.0);
    }

    swaption_price.max(0.0)
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
#[allow(clippy::expect_used, clippy::panic, clippy::indexing_slicing)]
mod tests {
    use super::*;

    /// Flat discount curve at a given continuously compounded rate.
    fn flat_df(rate: f64) -> impl Fn(f64) -> f64 {
        move |t: f64| (-rate * t).exp()
    }

    #[test]
    fn hw_params_validation() {
        assert!(HullWhiteParams::new(0.05, 0.01).is_ok());
        assert!(HullWhiteParams::new(0.0, 0.01).is_err()); // kappa = 0
        assert!(HullWhiteParams::new(-0.1, 0.01).is_err()); // kappa < 0
        assert!(HullWhiteParams::new(0.05, 0.0).is_err()); // sigma = 0
        assert!(HullWhiteParams::new(0.05, -0.01).is_err()); // sigma < 0
    }

    #[test]
    fn b_function_properties() {
        let p = HullWhiteParams::new(0.1, 0.01).expect("valid");
        let b = p.b_function(0.0, 1.0);
        // B(0, 1) = (1 − e^{−0.1}) / 0.1 ≈ 0.9516
        assert!((b - 0.9516).abs() < 0.001);

        // B should be positive and increasing in (t2 − t1)
        let b_short = p.b_function(0.0, 0.5);
        let b_long = p.b_function(0.0, 2.0);
        assert!(b_short < b);
        assert!(b < b_long);
    }

    #[test]
    fn bond_option_vol_positive() {
        let p = HullWhiteParams::new(0.05, 0.01).expect("valid");
        let vol = p.bond_option_vol(0.0, 1.0, 2.0);
        assert!(vol > 0.0, "Bond option vol should be positive: {vol}");
    }

    #[test]
    fn swaption_price_positive() {
        let df_fn = flat_df(0.03);
        let price = hw1f_swaption_price(0.05, 0.01, &df_fn, 1.0, 5.0, 0.03, 2);
        assert!(price > 0.0, "Swaption price should be positive: {price:.6}");
    }

    #[test]
    fn swaption_price_monotone_in_sigma() {
        let df_fn = flat_df(0.03);
        let fwd = {
            let (_, r) = compute_swap_annuity_and_rate(&df_fn, 1.0, 5.0, 2);
            r
        };
        let p_low = hw1f_swaption_price(0.05, 0.005, &df_fn, 1.0, 5.0, fwd, 2);
        let p_high = hw1f_swaption_price(0.05, 0.015, &df_fn, 1.0, 5.0, fwd, 2);
        assert!(
            p_high > p_low,
            "Higher sigma should give higher swaption price: {p_high:.6} vs {p_low:.6}"
        );
    }

    #[test]
    fn calibrate_hw1f_round_trip() {
        let true_kappa = 0.05;
        let true_sigma = 0.01;
        let rate = 0.03;
        let df_fn = flat_df(rate);
        let ppy = SwapFrequency::SemiAnnual.periods_per_year();

        let swaption_specs: Vec<(f64, f64)> =
            vec![(1.0, 5.0), (2.0, 5.0), (5.0, 5.0), (1.0, 10.0), (5.0, 10.0)];

        let quotes: Vec<SwaptionQuote> = swaption_specs
            .iter()
            .map(|&(expiry, tenor)| {
                let (annuity, fwd_rate) = compute_swap_annuity_and_rate(&df_fn, expiry, tenor, ppy);
                let model_price = hw1f_swaption_price(
                    true_kappa, true_sigma, &df_fn, expiry, tenor, fwd_rate, ppy,
                );

                let normal_vol = if annuity > 1e-15 && expiry > 0.0 {
                    let approx_vol =
                        model_price / (annuity * (expiry / (2.0 * std::f64::consts::PI)).sqrt());
                    approx_vol.max(1e-6)
                } else {
                    0.005
                };

                SwaptionQuote {
                    expiry,
                    tenor,
                    volatility: normal_vol,
                    is_normal_vol: true,
                }
            })
            .collect();

        let (params, report) =
            calibrate_hull_white_to_swaptions(&df_fn, &quotes).expect("Calibration should succeed");

        assert!(
            report.success,
            "Calibration should succeed: {}",
            report.convergence_reason
        );
        assert!(
            params.kappa > 0.0 && params.kappa < 1.0,
            "kappa should be reasonable: {:.4}",
            params.kappa
        );
        assert!(
            params.sigma > 0.0 && params.sigma < 0.1,
            "sigma should be reasonable: {:.4}",
            params.sigma
        );
    }

    #[test]
    fn calibrate_hw1f_annual_vs_semiannual_produces_different_params() {
        let df_fn = flat_df(0.03);
        let quotes = vec![
            SwaptionQuote {
                expiry: 1.0,
                tenor: 5.0,
                volatility: 0.005,
                is_normal_vol: true,
            },
            SwaptionQuote {
                expiry: 5.0,
                tenor: 5.0,
                volatility: 0.006,
                is_normal_vol: true,
            },
            SwaptionQuote {
                expiry: 10.0,
                tenor: 5.0,
                volatility: 0.005,
                is_normal_vol: true,
            },
        ];

        let (params_semi, _) = calibrate_hull_white_to_swaptions_with_frequency(
            &df_fn,
            &quotes,
            SwapFrequency::SemiAnnual,
        )
        .expect("semi-annual");
        let (params_ann, _) = calibrate_hull_white_to_swaptions_with_frequency(
            &df_fn,
            &quotes,
            SwapFrequency::Annual,
        )
        .expect("annual");

        assert!(
            (params_semi.kappa - params_ann.kappa).abs() > 1e-6
                || (params_semi.sigma - params_ann.sigma).abs() > 1e-6,
            "Different frequencies should produce different params: semi={:?} ann={:?}",
            params_semi,
            params_ann
        );
    }

    #[test]
    fn test_hw1f_brent_fallback_extreme_params() {
        let kappa = 5.0;
        let sigma = 0.03;
        let df = flat_df(0.03);

        let price = hw1f_swaption_price(kappa, sigma, &df, 1.0, 5.0, 0.03, 2);
        assert!(
            price.is_finite(),
            "Swaption price should be finite with Brent fallback"
        );
        assert!(price >= 0.0, "Swaption price must be non-negative");
    }

    #[test]
    fn calibrate_hw1f_rejects_insufficient_quotes() {
        let quotes = vec![SwaptionQuote {
            expiry: 1.0,
            tenor: 5.0,
            volatility: 0.005,
            is_normal_vol: true,
        }];
        let df_fn = flat_df(0.03);
        let result = calibrate_hull_white_to_swaptions(&df_fn, &quotes);
        assert!(result.is_err(), "Should reject < 2 quotes");
    }
}
