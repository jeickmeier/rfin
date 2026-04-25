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
use crate::instruments::common_impl::models::trees::HullWhiteTreeConfig;

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

impl Default for HullWhiteParams {
    /// Returns generic default parameters for testing and initialization.
    ///
    /// These defaults (κ=3%, σ=1%) are not calibrated and should not be used
    /// for production pricing without an explicit calibration decision.
    fn default() -> Self {
        Self {
            kappa: 0.03,
            sigma: 0.01,
        }
    }
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

    /// Returns true when these parameters are the generic uncalibrated defaults.
    #[must_use]
    pub fn is_uncalibrated_default(&self) -> bool {
        (self.kappa - 0.03).abs() < f64::EPSILON && (self.sigma - 0.01).abs() < f64::EPSILON
    }

    /// Create tree configuration with the specified number of steps.
    pub(crate) fn tree_config(&self, steps: usize) -> HullWhiteTreeConfig {
        HullWhiteTreeConfig::new(self.kappa, self.sigma, steps)
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

impl SwaptionQuote {
    /// Construct a validated swaption market quote.
    pub fn try_new(
        expiry: f64,
        tenor: f64,
        volatility: f64,
        is_normal_vol: bool,
    ) -> finstack_core::Result<Self> {
        if !expiry.is_finite() || expiry <= 0.0 {
            return Err(finstack_core::Error::Validation(format!(
                "Swaption expiry must be positive, got {expiry}"
            )));
        }
        if !tenor.is_finite() || tenor <= 0.0 {
            return Err(finstack_core::Error::Validation(format!(
                "Swaption tenor must be positive, got {tenor}"
            )));
        }
        if !volatility.is_finite() || volatility <= 0.0 {
            return Err(finstack_core::Error::Validation(format!(
                "Swaption volatility must be positive, got {volatility}"
            )));
        }
        Ok(Self {
            expiry,
            tenor,
            volatility,
            is_normal_vol,
        })
    }
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
    pub(crate) fn periods_per_year(self) -> usize {
        match self {
            Self::Annual => 1,
            Self::SemiAnnual => 2,
            Self::Quarterly => 4,
        }
    }
}

/// Calibrate Hull-White 1-factor parameters to European swaption market data.
///
/// Fits κ (mean reversion) and σ (short rate volatility) by minimising
/// squared differences between model and market swaption prices.
///
/// # Arguments
///
/// * `df` - Discount factor function: `df(t)` returns P(0, t). Must satisfy `df(0) ≈ 1`.
/// * `quotes` - Swaption market data.
/// * `frequency` - Coupon frequency of the underlying swap (e.g., semi-annual for USD,
///   annual for EUR). This materially affects the annuity factor and forward swap rate.
/// * `initial_guess` - Optional seed for (κ, σ). Pass `None` to use built-in defaults.
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
///     calibrate_hull_white_to_swaptions, SwaptionQuote, SwapFrequency,
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
/// let (params, report) = calibrate_hull_white_to_swaptions(
///     &df, &quotes, SwapFrequency::SemiAnnual, None,
/// ).unwrap();
/// assert!(report.success);
/// ```
// The borrowed-closure form `&residuals` below is intentional: the
// inner `solve_system_with_dim_stats` moves its `Res: Fn(...)` argument
// by value, so we must re-borrow for each multi-start iteration. Clippy
// flags the `&` as needless because `&F: Fn(...)` and `F: Fn(...)` are
// both acceptable for the generic — but dropping the `&` would move the
// closure on the first call.
#[allow(clippy::needless_borrows_for_generic_args)]
pub fn calibrate_hull_white_to_swaptions(
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
    // Per-quote vega weights (∂price/∂σ) used to convert price
    // residuals into dimensionless vol-residuals. The vega-weighted
    // form prevents long-dated quotes (large annuities → large prices)
    // from dominating the objective and pushing κ → 0 on mixed-expiry
    // co-terminal grids.
    let mut vegas = Vec::with_capacity(n_quotes);
    // Vega floor: 1 bp of annuity-year. Protects against division by a
    // near-zero vega at extreme expiries or zero quoted vol.
    const VEGA_FLOOR: f64 = 1e-8;

    for q in quotes {
        let (annuity, fwd_rate) = compute_swap_annuity_and_rate(df, q.expiry, q.tenor, ppy);
        let market_price = compute_swaption_market_price(
            annuity,
            fwd_rate,
            q.expiry,
            q.volatility,
            q.is_normal_vol,
        );
        let vega = swaption_atm_vega(annuity, fwd_rate, q.expiry, q.volatility, q.is_normal_vol)
            .max(VEGA_FLOOR);
        market_prices.push(market_price);
        annuities.push(annuity);
        fwd_swap_rates.push(fwd_rate);
        vegas.push(vega);
    }

    let (default_kappa_init, default_sigma_init) = infer_hw_initial_guess(quotes, &fwd_swap_rates);
    let kappa_init: f64 = initial_guess.map(|p| p.kappa).unwrap_or(default_kappa_init);
    let sigma_init: f64 = initial_guess.map(|p| p.sigma).unwrap_or(default_sigma_init);
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
                // Vega-weighted price residual: algebraically the
                // first-order approximation to (σ_model − σ_market), so
                // all quotes enter the objective on an implied-vol scale
                // rather than a price scale. Gilli–Maringer–Schumann
                // §13.4 prescribes exactly this form for industry-grade
                // HW1F calibration.
                resid[idx] = (model_price - market_prices[idx]) / vegas[idx];
            } else {
                resid[idx] = 1e6;
            }
        }
    };

    let solver = LevenbergMarquardtSolver::new()
        .with_tolerance(1e-12)
        .with_max_iterations(300);

    // Initial solve from the nominal guess.
    let initial_solution = solver.solve_system_with_dim_stats(&residuals, &x0, n_quotes)?;

    // Halton multi-start: 5 deterministic restarts around x0 with 50%
    // perturbation scale. Keeps the solution with the lowest weighted
    // residual norm, escaping local minima that a single LM run is
    // prone to on HW1F's (κ, σ) objective surface.
    use crate::calibration::solver::multi_start::perturb_initial_guess;
    const NUM_RESTARTS: usize = 5;
    const PERTURB_SCALE: f64 = 0.5;

    let initial_norm = weighted_residual_norm(&initial_solution.params, &residuals, n_quotes);
    let mut best_solution = initial_solution;
    let mut best_norm = initial_norm;

    for restart_idx in 0..NUM_RESTARTS {
        let perturbed = perturb_initial_guess(&x0, PERTURB_SCALE, restart_idx, None, None);
        let probe_x0 = [perturbed[0], perturbed[1]];
        if let Ok(sol) = solver.solve_system_with_dim_stats(&residuals, &probe_x0, n_quotes) {
            let norm = weighted_residual_norm(&sol.params, &residuals, n_quotes);
            if norm.is_finite() && norm < best_norm {
                best_norm = norm;
                best_solution = sol;
            }
        }
    }

    let solution = best_solution;
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
    .with_model_version(finstack_core::versions::HULL_WHITE_1F)
    .with_metadata("kappa", format!("{kappa:.6}"))
    .with_metadata("sigma", format!("{sigma:.6}"))
    .with_metadata("initial_kappa", format!("{kappa_init:.6}"))
    .with_metadata("initial_sigma", format!("{sigma_init:.6}"))
    .with_metadata("multi_start_restarts", NUM_RESTARTS.to_string())
    .with_metadata(
        "residual_weighting",
        "1/vega (vega-weighted price residual)".to_string(),
    )
    .with_metadata("swap_frequency", format!("{frequency:?}"));

    // κ hard-bounds check: mean-reversion must lie in [1e-3, 1.0].
    // Below 0.001 the half-life exceeds 693y and tree/bond price
    // calculations become numerically unstable; above 1.0 the model
    // effectively collapses to the instantaneous-rate level and no
    // longer has meaningful term structure.
    const KAPPA_MIN: f64 = 0.001;
    const KAPPA_MAX: f64 = 1.0;
    if !(KAPPA_MIN..=KAPPA_MAX).contains(&kappa) {
        return Err(finstack_core::Error::Validation(format!(
            "Hull-White calibration produced κ = {kappa:.6} outside the \
             bounded range [{KAPPA_MIN}, {KAPPA_MAX}]. This typically \
             indicates an under-weighted, over-damped, or under-specified \
             swaption grid; review the quotes or supply a bounded \
             `initial_guess`."
        )));
    }

    let params = HullWhiteParams::new(kappa, sigma)?;

    Ok((params, report))
}

/// ATM vega for a swaption expressed in the same volatility units as the
/// quote (Bachelier σ for normal vol, Black-76 σ for lognormal).
///
/// Used as the per-quote weight in the vega-weighted price residual; see
/// the module-level note in `calibrate_hull_white_to_swaptions`.
fn swaption_atm_vega(annuity: f64, fwd_rate: f64, expiry: f64, vol: f64, is_normal: bool) -> f64 {
    if is_normal {
        annuity * finstack_core::math::volatility::bachelier_vega(fwd_rate, fwd_rate, vol, expiry)
    } else {
        annuity * finstack_core::math::volatility::black_vega(fwd_rate, fwd_rate, vol, expiry)
    }
}

/// Compute √(Σ r_i²) for the vega-weighted residual vector at the given
/// parameter vector. Used by the multi-start loop to compare candidates.
fn weighted_residual_norm<F>(params: &[f64], residuals: &F, n: usize) -> f64
where
    F: Fn(&[f64], &mut [f64]),
{
    let mut buf = vec![0.0_f64; n];
    residuals(params, &mut buf);
    buf.iter()
        .map(|r| if r.is_finite() { r * r } else { 1e18 })
        .sum::<f64>()
        .sqrt()
}

// =============================================================================
// Futures convexity adjustment
// =============================================================================

/// Compute the Hull-White 1-factor futures convexity adjustment.
///
/// Returns the adjustment (in rate terms) to convert a futures rate to a forward rate:
/// `forward = futures_rate - convexity_adjustment`.
///
/// The formula (Hull, 6th ed., Chapter 6):
///
/// $$
/// \text{CA} = \tfrac{1}{2} \, \sigma^2 \, B(0, T_1) \, B(T_1, T_2)
/// $$
///
/// where:
/// - $T_1$ = futures settlement time (years from today)
/// - $T_2$ = futures end time (maturity, years from today)
/// - $\sigma$ = HW1F short-rate volatility
/// - $\kappa$ = HW1F mean-reversion speed
/// - $B(t_1, t_2) = (1 - e^{-\kappa(t_2 - t_1)}) / \kappa$
///
/// # Arguments
/// * `kappa` - Mean-reversion speed
/// * `sigma` - Short-rate volatility
/// * `t_settle` - Settlement time in years ($T_1$)
/// * `t_end` - End/maturity time in years ($T_2$)
///
/// # Returns
/// The convexity adjustment in the same rate units as sigma.
pub fn hw1f_convexity_adjustment(kappa: f64, sigma: f64, t_settle: f64, t_end: f64) -> f64 {
    let b_0s = hw_b(kappa, 0.0, t_settle);
    let b_se = hw_b(kappa, t_settle, t_end);
    0.5 * sigma * sigma * b_0s * b_se
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
pub(crate) fn compute_swap_annuity_and_rate(
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
        let p0 = df(t0).max(1e-12);
        let p_n = df(t_n).max(1e-12);
        ((p0 / p_n).ln() / tenor.max(1e-8)).max(0.0)
    };

    (annuity, fwd_rate)
}

fn infer_hw_initial_guess(quotes: &[SwaptionQuote], fwd_swap_rates: &[f64]) -> (f64, f64) {
    let horizon = if quotes.is_empty() {
        5.0
    } else {
        quotes.iter().map(|q| q.expiry + 0.5 * q.tenor).sum::<f64>() / quotes.len() as f64
    };
    let avg_vol = if quotes.is_empty() {
        0.01
    } else {
        quotes.iter().map(|q| q.volatility.abs()).sum::<f64>() / quotes.len() as f64
    };
    let avg_fwd = if fwd_swap_rates.is_empty() {
        0.02
    } else {
        fwd_swap_rates.iter().map(|r| r.abs()).sum::<f64>() / fwd_swap_rates.len() as f64
    };

    let kappa_init = (1.0 / horizon.max(0.5)).clamp(0.01, 0.30);
    let sigma_init = (avg_vol * avg_fwd.max(0.005)).clamp(0.001, 0.05);
    (kappa_init, sigma_init)
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
pub(crate) fn hw1f_swaption_price(
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
        let bracket_lo = f0t0 - 0.20;
        let bracket_hi = f0t0 + 0.20;
        let brent = BrentSolver::new()
            .tolerance(1e-12)
            .bracket_bounds(bracket_lo, bracket_hi);
        match brent.solve(g, f0t0) {
            Ok(r) => r_star = r,
            Err(_) => {
                tracing::warn!("HW1F r* Brent fallback also failed; returning NaN");
                r_star = f64::NAN;
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
            calibrate_hull_white_to_swaptions(&df_fn, &quotes, SwapFrequency::default(), None)
                .expect("Calibration should succeed");

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

        let (params_semi, _) =
            calibrate_hull_white_to_swaptions(&df_fn, &quotes, SwapFrequency::SemiAnnual, None)
                .expect("semi-annual");
        let (params_ann, _) =
            calibrate_hull_white_to_swaptions(&df_fn, &quotes, SwapFrequency::Annual, None)
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
        let result =
            calibrate_hull_white_to_swaptions(&df_fn, &quotes, SwapFrequency::default(), None);
        assert!(result.is_err(), "Should reject < 2 quotes");
    }

    // ========================================================================
    // HW1F vega-weighted calibration + multi-start
    // ========================================================================

    /// Wide-grid round-trip: generate ATM normal vols from a known
    /// `(κ*, σ*) = (0.08, 0.012)` on a 10-swaption co-terminal-style
    /// grid spanning 1Y to 10Y expiries × 5Y and 10Y tenors, then verify
    /// the calibrator recovers κ in a tight neighbourhood of κ*.
    ///
    /// Pre-fix: the **unweighted** price residual let the 10Y×10Y quote
    /// (largest annuity → largest price) dominate the objective; the LM
    /// solver minimised overall price error by pushing κ toward zero
    /// (which widens the long-dated bond-option vol and soaks up most of
    /// the residual) at the cost of a 20–30 bp vol error on the 1Y
    /// quotes. The vega-weighted residual (post-fix) puts every quote
    /// on an implied-vol scale and multi-start escapes the flat κ→0
    /// region of the objective surface.
    #[test]
    fn hw1f_calibration_recovers_kappa_on_wide_round_trip_grid() {
        let true_kappa = 0.08_f64;
        let true_sigma = 0.012_f64;
        let df_fn = flat_df(0.03);
        let ppy = SwapFrequency::SemiAnnual.periods_per_year();

        // 10-swaption co-terminal grid.
        let specs: &[(f64, f64)] = &[
            (1.0, 5.0),
            (2.0, 5.0),
            (3.0, 5.0),
            (5.0, 5.0),
            (7.0, 5.0),
            (10.0, 5.0),
            (1.0, 10.0),
            (3.0, 10.0),
            (5.0, 10.0),
            (10.0, 10.0),
        ];

        // Back out the implied normal vol from the model price so the
        // resulting quotes are internally consistent with (κ*, σ*). Use
        // the Bachelier ATM relation: price ≈ annuity · σ_n · √T / √(2π).
        let quotes: Vec<SwaptionQuote> = specs
            .iter()
            .map(|&(expiry, tenor)| {
                let (annuity, fwd_rate) = compute_swap_annuity_and_rate(&df_fn, expiry, tenor, ppy);
                let model_price = hw1f_swaption_price(
                    true_kappa, true_sigma, &df_fn, expiry, tenor, fwd_rate, ppy,
                );
                let vol = model_price / (annuity * (expiry / (2.0 * std::f64::consts::PI)).sqrt());
                SwaptionQuote {
                    expiry,
                    tenor,
                    volatility: vol.max(1e-6),
                    is_normal_vol: true,
                }
            })
            .collect();

        let (params, report) =
            calibrate_hull_white_to_swaptions(&df_fn, &quotes, SwapFrequency::SemiAnnual, None)
                .expect("calibration should succeed");

        assert!(
            report.success,
            "calibration should converge, got: {}",
            report.convergence_reason
        );

        // Recovery tolerance: κ within 20% of the true value — tight
        // enough to fail pre-fix (where the unweighted residual pulled κ
        // into the single-digit-bp range) but permissive enough to
        // accommodate the LM convergence tolerance and multi-start
        // noise.
        assert!(
            (true_kappa * 0.8..=true_kappa * 1.2).contains(&params.kappa),
            "κ = {:.6} not within 20% of κ* = {true_kappa:.6}; \
             pre-fix C8 behaviour was to push κ toward zero on wide \
             expiry grids because the unweighted price residual let \
             long-dated quotes dominate",
            params.kappa
        );
        assert!(
            (true_sigma * 0.5..=true_sigma * 1.5).contains(&params.sigma),
            "σ = {:.6} not within 50% of σ* = {true_sigma:.6}",
            params.sigma
        );
    }

    /// κ out of bounds `[0.001, 1.0]` must return `Err` rather than a
    /// `tracing::warn!`-and-succeed. Use synthetic quotes with
    /// inconsistent rate/tenor structure to push the calibration to a
    /// pathological κ if it converges at all.
    #[test]
    fn hw1f_calibration_errors_when_kappa_drives_out_of_bounds() {
        // Construct pathological quotes: essentially flat very low vol
        // across a wide expiry grid. The LM will tend toward κ → 0 +
        // σ → 0; the post-fix implementation should either (a) find a
        // feasible κ in-bounds thanks to multi-start or (b) return an
        // OutOfBounds error. Both outcomes are acceptable; a silent
        // warn-and-return path is NOT.
        let df_fn = flat_df(0.03);
        let quotes: Vec<SwaptionQuote> = (1..=10)
            .map(|i| SwaptionQuote {
                expiry: i as f64,
                tenor: 5.0,
                volatility: 1e-6, // ~0 bp
                is_normal_vol: true,
            })
            .collect();

        let result =
            calibrate_hull_white_to_swaptions(&df_fn, &quotes, SwapFrequency::SemiAnnual, None);

        match result {
            Ok((params, _)) => {
                assert!(
                    (0.001..=1.0).contains(&params.kappa),
                    "κ = {:.6} outside hard bounds [0.001, 1.0]; Err expected \
                     rather than a warn-and-succeed path",
                    params.kappa
                );
            }
            Err(e) => {
                let msg = format!("{e}");
                assert!(
                    msg.contains("κ") || msg.contains("kappa") || msg.contains("bounded"),
                    "error message must identify κ-bounds violation: {msg}"
                );
            }
        }
    }
}
