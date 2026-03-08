//! Monte Carlo Option-Adjusted Spread (OAS) for agency MBS.
//!
//! Computes OAS using stochastic interest rate paths (Hull-White 1-factor)
//! with rate-dependent prepayment speeds, following the market-standard
//! approach used by Bloomberg, QuantLib, and other professional systems.
//!
//! # Methodology
//!
//! 1. Simulate N interest rate paths using HW1F exact discretization
//! 2. For each path, project cashflows with rate-dependent prepayment
//! 3. Discount each path's cashflows at the simulated short rates + OAS
//! 4. Average across paths to get the model price
//! 5. Use Brent's method to find OAS that equates model price to market price
//!
//! # Prepayment Model
//!
//! The standard PSA model is modified with a rate-dependent multiplier:
//! - When rates fall (refinancing incentive), prepayment speeds increase
//! - When rates rise, prepayment speeds decrease (lock-in effect)
//!
//! The multiplier is:
//! ```text
//! multiplier = exp(-β × (rate - base_rate))
//! ```
//! where β controls the sensitivity (typical: 5.0-10.0).
//!
//! # References
//!
//! - Fabozzi, F. J. (2016). *Bond Markets, Analysis, and Strategies*. Pearson.
//! - Hayre, L. (2001). *Salomon Smith Barney Guide to Mortgage-Backed and
//!   Asset-Backed Securities*. John Wiley & Sons.
//! - O'Kane, D. (2008). *Modelling Single-name and Multi-name Credit
//!   Derivatives*. John Wiley & Sons.

use crate::instruments::fixed_income::mbs_passthrough::AgencyMbsPassthrough;
use finstack_core::dates::Date;
use finstack_core::market_data::context::MarketContext;
use finstack_core::math::solver::{BrentSolver, Solver};
use finstack_core::Result;

/// Configuration for Monte Carlo OAS calculation.
#[derive(Debug, Clone)]
pub struct McOasConfig {
    /// Number of simulation paths (default: 512).
    pub num_paths: usize,
    /// Number of monthly time steps per path (default: WAM).
    /// If None, uses the MBS WAM.
    pub num_steps: Option<usize>,
    /// Hull-White mean reversion speed κ (default: 0.05).
    pub hw_kappa: f64,
    /// Hull-White short-rate volatility σ (default: 0.01).
    pub hw_sigma: f64,
    /// Prepayment rate sensitivity to interest rates β (default: 7.0).
    /// Higher values make prepayment more sensitive to rate changes.
    pub prepay_rate_sensitivity: f64,
    /// Random seed for reproducibility (default: 42).
    pub seed: u64,
    /// Solver tolerance for OAS root-finding (default: 1e-7).
    pub tolerance: f64,
}

impl Default for McOasConfig {
    fn default() -> Self {
        Self {
            num_paths: 512,
            num_steps: None,
            hw_kappa: 0.05,
            hw_sigma: 0.01,
            prepay_rate_sensitivity: 7.0,
            seed: 42,
            tolerance: 1e-7,
        }
    }
}

/// Result of a Monte Carlo OAS calculation.
#[derive(Debug, Clone)]
pub struct McOasResult {
    /// Option-adjusted spread in decimal (e.g., 0.01 for 100 bps).
    pub oas: f64,
    /// Average model price across all paths at the calculated OAS.
    pub model_price: f64,
    /// Target (market) price.
    pub market_price: f64,
    /// Price error at the solution.
    pub price_error: f64,
    /// Number of simulation paths used.
    pub num_paths: usize,
    /// Whether the solver converged.
    pub converged: bool,
    /// Standard error of the price estimate across paths.
    pub price_std_error: f64,
}

/// A single simulated short-rate path.
struct RatePath {
    /// Monthly short rates along the path.
    rates: Vec<f64>,
}

/// Simulate Hull-White 1-factor short rate paths.
///
/// Uses exact discretization (analytical conditional distribution)
/// for the OU/HW1F process:
/// ```text
/// r_{t+Δt} = r_t × e^{-κΔt} + θ(1 - e^{-κΔt}) + σ√[(1-e^{-2κΔt})/(2κ)] × Z
/// ```
fn simulate_rate_paths(
    initial_rate: f64,
    kappa: f64,
    sigma: f64,
    theta: f64,
    num_paths: usize,
    num_steps: usize,
    seed: u64,
) -> Vec<RatePath> {
    let dt = 1.0 / 12.0; // Monthly steps
    let exp_kappa_dt = (-kappa * dt).exp();
    let drift_coeff = theta * (1.0 - exp_kappa_dt);

    // Conditional std dev of r_{t+Δt} | r_t
    let std_dev = if (kappa * dt).abs() < 1e-8 {
        sigma * dt.sqrt()
    } else {
        sigma * ((1.0 - (-2.0 * kappa * dt).exp()) / (2.0 * kappa)).sqrt()
    };

    let mut paths = Vec::with_capacity(num_paths);

    for path_idx in 0..num_paths {
        // Simple deterministic seeded RNG (Xoshiro-like)
        let mut state = seed
            .wrapping_add(path_idx as u64)
            .wrapping_mul(6364136223846793005);

        let mut rates = Vec::with_capacity(num_steps + 1);
        rates.push(initial_rate);

        let mut r = initial_rate;

        for _step in 0..num_steps {
            // Generate standard normal via Box-Muller
            state = state
                .wrapping_mul(6364136223846793005)
                .wrapping_add(1442695040888963407);
            let u1 = (state >> 11) as f64 / (1u64 << 53) as f64;
            state = state
                .wrapping_mul(6364136223846793005)
                .wrapping_add(1442695040888963407);
            let u2 = (state >> 11) as f64 / (1u64 << 53) as f64;

            let u1_safe = u1.max(1e-15);
            let z = (-2.0 * u1_safe.ln()).sqrt() * (2.0 * std::f64::consts::PI * u2).cos();

            // Exact HW1F step
            r = r * exp_kappa_dt + drift_coeff + std_dev * z;

            rates.push(r);
        }

        paths.push(RatePath { rates });
    }

    paths
}

/// Compute rate-dependent SMM (Single Monthly Mortality) from the base PSA model.
///
/// The base SMM from the PSA model is adjusted by a multiplier that depends on
/// the current short rate relative to the base rate:
///
/// ```text
/// adjusted_smm = base_smm × exp(-β × (current_rate - base_rate))
/// ```
///
/// This captures the refinancing incentive: lower rates → faster prepayment.
fn rate_adjusted_smm(base_smm: f64, current_rate: f64, base_rate: f64, sensitivity: f64) -> f64 {
    let multiplier = (-sensitivity * (current_rate - base_rate)).exp();
    // Cap at 0.9999 to prevent full balance prepayment in a single period
    (base_smm * multiplier).clamp(0.0, 0.9999)
}

/// Price MBS on a single rate path with a given OAS.
///
/// Projects monthly cashflows using rate-dependent prepayment and
/// discounts each cashflow at the path's short rate + OAS.
fn price_on_path(
    mbs: &AgencyMbsPassthrough,
    path: &RatePath,
    base_rate: f64,
    oas: f64,
    prepay_sensitivity: f64,
) -> f64 {
    let monthly_coupon_rate = mbs.pass_through_rate / 12.0;
    let monthly_mortgage_rate = mbs.wac / 12.0;
    let dt = 1.0 / 12.0;

    let mut balance = mbs.current_face.amount();
    let mut pv = 0.0;
    let mut cumulative_df = 1.0;

    let wam = mbs.wam as usize;
    let num_steps = path.rates.len().saturating_sub(1).min(wam);

    for month in 0..num_steps {
        if balance < 0.01 {
            break;
        }

        let current_rate = path.rates[month + 1];

        // Discount factor for this step: exp(-(r + oas) × dt)
        let step_df = (-(current_rate + oas) * dt).exp();
        cumulative_df *= step_df;

        // Seasoning for base PSA SMM
        let seasoning = mbs.seasoning_months(mbs.issue_date) + month as u32 + 1;
        // CPR is always non-negative for valid MBS prepayment models,
        // so the Result is always Ok. Use unwrap_or(0.0) as a conservative
        // fallback to avoid panic in Monte Carlo hot path.
        let base_smm = mbs.prepayment_model.smm(seasoning).unwrap_or(0.0);

        // Rate-adjusted SMM
        let smm = rate_adjusted_smm(base_smm, current_rate, base_rate, prepay_sensitivity);

        // Scheduled amortization
        let remaining = wam.saturating_sub(month + 1).max(1);
        let scheduled_principal = if remaining <= 1 {
            balance
        } else if monthly_mortgage_rate > 1e-12 {
            let factor = (1.0 + monthly_mortgage_rate).powi(remaining as i32);
            let payment = balance * monthly_mortgage_rate * factor / (factor - 1.0);
            let interest_part = balance * monthly_mortgage_rate;
            (payment - interest_part).max(0.0).min(balance)
        } else {
            balance / remaining as f64
        };

        // Prepayment
        let prepayment = balance * smm;

        // Interest
        let interest = balance * monthly_coupon_rate;

        // Total cashflow
        let total_cf = scheduled_principal + prepayment + interest;

        // PV of this month's cashflow
        pv += total_cf * cumulative_df;

        // Update balance
        balance = (balance - scheduled_principal - prepayment).max(0.0);
    }

    pv
}

/// Calculate Monte Carlo OAS for an agency MBS.
///
/// Uses stochastic interest rate paths with rate-dependent prepayment to
/// compute the OAS that equates the average discounted cashflow to the
/// market price.
///
/// # Arguments
///
/// * `mbs` - Agency MBS passthrough instrument
/// * `market_price_pct` - Market price as percentage of face (e.g., 98.5)
/// * `market` - Market context with discount curves
/// * `as_of` - Valuation date
/// * `config` - Monte Carlo configuration (paths, HW params, seed)
///
/// # Returns
///
/// Monte Carlo OAS result with spread, convergence, and standard error.
///
/// # Example
///
/// ```text
/// use finstack_valuations::instruments::fixed_income::mbs_passthrough::{
///     AgencyMbsPassthrough,
///     metrics::mc_oas::{calculate_mc_oas, McOasConfig},
/// };
///
/// let mbs = AgencyMbsPassthrough::example().unwrap();
/// let config = McOasConfig { num_paths: 1024, ..Default::default() };
/// let result = calculate_mc_oas(&mbs, 98.5, &market, as_of, &config)?;
/// println!("MC OAS: {:.0} bps", result.oas * 10_000.0);
/// ```
pub fn calculate_mc_oas(
    mbs: &AgencyMbsPassthrough,
    market_price_pct: f64,
    market: &MarketContext,
    _as_of: Date,
    config: &McOasConfig,
) -> Result<McOasResult> {
    let market_price = market_price_pct / 100.0 * mbs.current_face.amount();

    // Extract initial short rate from discount curve
    let discount_curve = market.get_discount(&mbs.discount_curve_id)?;
    let initial_rate = {
        let t = 1.0 / 12.0; // 1-month rate
        let df = discount_curve.df(t);
        if df > 0.0 {
            -df.ln() / t
        } else {
            0.03
        }
    };

    // θ for HW1F (long-run mean) = implied from the curve at ~5Y
    let theta = {
        let t = 5.0;
        let df = discount_curve.df(t);
        if df > 0.0 {
            -df.ln() / t
        } else {
            initial_rate
        }
    };

    let num_steps = config.num_steps.unwrap_or(mbs.wam as usize);

    // Simulate rate paths
    let paths = simulate_rate_paths(
        initial_rate,
        config.hw_kappa,
        config.hw_sigma,
        theta,
        config.num_paths,
        num_steps,
        config.seed,
    );

    // Objective: average price across paths minus market price
    let objective = |oas: f64| -> f64 {
        let total: f64 = paths
            .iter()
            .map(|path| price_on_path(mbs, path, initial_rate, oas, config.prepay_rate_sensitivity))
            .sum();
        let avg_price = total / config.num_paths as f64;
        avg_price - market_price
    };

    // Solve for OAS using Brent's method
    let solver = BrentSolver::new()
        .tolerance(config.tolerance)
        .max_iterations(200)
        .bracket_bounds(-0.10, 0.20)
        .initial_bracket_size(Some(0.05));

    let result = solver.solve(objective, 0.0);

    // Compute final statistics at the solved OAS
    let oas = match &result {
        Ok(oas) => *oas,
        Err(_) => 0.0,
    };

    let path_prices: Vec<f64> = paths
        .iter()
        .map(|path| price_on_path(mbs, path, initial_rate, oas, config.prepay_rate_sensitivity))
        .collect();

    let avg_price = path_prices.iter().sum::<f64>() / config.num_paths as f64;

    // Standard error of the mean
    let variance = if config.num_paths > 1 {
        let mean = avg_price;
        path_prices.iter().map(|p| (p - mean).powi(2)).sum::<f64>() / (config.num_paths - 1) as f64
    } else {
        0.0
    };
    let std_error = (variance / config.num_paths as f64).sqrt();

    Ok(McOasResult {
        oas,
        model_price: avg_price,
        market_price,
        price_error: avg_price - market_price,
        num_paths: config.num_paths,
        converged: result.is_ok(),
        price_std_error: std_error,
    })
}

#[cfg(test)]
#[allow(clippy::expect_used, clippy::panic)]
mod tests {
    use super::*;
    use crate::cashflow::builder::specs::PrepaymentModelSpec;
    use crate::instruments::fixed_income::mbs_passthrough::{AgencyProgram, PoolType};
    use finstack_core::currency::Currency;
    use finstack_core::dates::DayCount;
    use finstack_core::market_data::term_structures::DiscountCurve;
    use finstack_core::math::interp::InterpStyle;
    use finstack_core::money::Money;
    use finstack_core::types::{CurveId, InstrumentId};
    use time::Month;

    fn create_test_mbs() -> AgencyMbsPassthrough {
        AgencyMbsPassthrough::builder()
            .id(InstrumentId::new("TEST-MBS-MC"))
            .pool_id("TEST-POOL".into())
            .agency(AgencyProgram::Fnma)
            .pool_type(PoolType::Generic)
            .original_face(Money::new(1_000_000.0, Currency::USD))
            .current_face(Money::new(1_000_000.0, Currency::USD))
            .current_factor(1.0)
            .wac(0.045)
            .pass_through_rate(0.04)
            .servicing_fee_rate(0.0025)
            .guarantee_fee_rate(0.0025)
            .wam(360)
            .issue_date(Date::from_calendar_date(2024, Month::January, 1).expect("valid"))
            .maturity(Date::from_calendar_date(2054, Month::January, 1).expect("valid"))
            .prepayment_model(PrepaymentModelSpec::psa(1.0))
            .discount_curve_id(CurveId::new("USD-OIS"))
            .day_count(DayCount::Thirty360)
            .build()
            .expect("valid mbs")
    }

    fn create_test_market(as_of: Date) -> MarketContext {
        let disc = DiscountCurve::builder("USD-OIS")
            .base_date(as_of)
            .knots([
                (0.0, 1.0),
                (1.0, 0.96),
                (5.0, 0.80),
                (10.0, 0.60),
                (30.0, 0.30),
            ])
            .interp(InterpStyle::Linear)
            .build()
            .expect("valid curve");

        MarketContext::new().insert(disc)
    }

    #[test]
    fn test_rate_path_simulation() {
        let paths = simulate_rate_paths(0.04, 0.05, 0.01, 0.04, 100, 120, 42);
        assert_eq!(paths.len(), 100);

        for path in &paths {
            assert_eq!(path.rates.len(), 121); // 120 steps + initial
                                               // Initial rate should match
            assert!((path.rates[0] - 0.04).abs() < 1e-10);
            // Rates should be finite
            for &r in &path.rates {
                assert!(r.is_finite());
            }
        }
    }

    #[test]
    fn test_rate_adjusted_smm() {
        let base_smm = 0.005;
        let base_rate = 0.04;

        // Same rate → multiplier ≈ 1
        let adj = rate_adjusted_smm(base_smm, 0.04, base_rate, 7.0);
        assert!((adj - base_smm).abs() < 1e-10);

        // Lower rate → faster prepayment
        let adj_low = rate_adjusted_smm(base_smm, 0.02, base_rate, 7.0);
        assert!(adj_low > base_smm);

        // Higher rate → slower prepayment
        let adj_high = rate_adjusted_smm(base_smm, 0.06, base_rate, 7.0);
        assert!(adj_high < base_smm);

        // SMM should be capped at 0.9999
        let extreme = rate_adjusted_smm(0.5, -0.10, base_rate, 20.0);
        assert!(extreme <= 0.9999);
    }

    #[test]
    fn test_mc_oas_at_model_price() {
        let mbs = create_test_mbs();
        let as_of = Date::from_calendar_date(2024, Month::January, 15).expect("valid");
        let market = create_test_market(as_of);

        // First, compute the model price at OAS = 0
        let config = McOasConfig {
            num_paths: 64, // Fewer paths for speed in test
            ..Default::default()
        };

        let paths = simulate_rate_paths(0.04, config.hw_kappa, config.hw_sigma, 0.04, 64, 360, 42);
        let avg_price: f64 = paths
            .iter()
            .map(|path| price_on_path(&mbs, path, 0.04, 0.0, config.prepay_rate_sensitivity))
            .sum::<f64>()
            / 64.0;

        let market_price_pct = avg_price / mbs.current_face.amount() * 100.0;

        // MC OAS at model price should be approximately 0
        let result =
            calculate_mc_oas(&mbs, market_price_pct, &market, as_of, &config).expect("mc oas");

        // Allow wider tolerance due to MC noise
        assert!(
            result.oas.abs() < 0.005,
            "OAS should be near zero at model price, got {}",
            result.oas
        );
    }

    #[test]
    fn test_mc_oas_discount_gives_positive_spread() {
        let mbs = create_test_mbs();
        let as_of = Date::from_calendar_date(2024, Month::January, 15).expect("valid");
        let market = create_test_market(as_of);

        let config = McOasConfig {
            num_paths: 64,
            ..Default::default()
        };

        // Discount price should give positive OAS
        let result = calculate_mc_oas(&mbs, 80.0, &market, as_of, &config).expect("mc oas");

        assert!(
            result.oas > 0.0,
            "OAS should be positive for discount price, got {}",
            result.oas
        );
    }

    #[test]
    fn test_mc_oas_deterministic_with_seed() {
        let mbs = create_test_mbs();
        let as_of = Date::from_calendar_date(2024, Month::January, 15).expect("valid");
        let market = create_test_market(as_of);

        let config = McOasConfig {
            num_paths: 32,
            seed: 12345,
            ..Default::default()
        };

        let result1 = calculate_mc_oas(&mbs, 95.0, &market, as_of, &config).expect("mc oas 1");
        let result2 = calculate_mc_oas(&mbs, 95.0, &market, as_of, &config).expect("mc oas 2");

        // Same seed should give identical results
        assert!(
            (result1.oas - result2.oas).abs() < 1e-12,
            "Same seed should give identical OAS"
        );
    }
}
