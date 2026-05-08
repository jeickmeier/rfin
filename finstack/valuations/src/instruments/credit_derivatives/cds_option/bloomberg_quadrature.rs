//! Bloomberg CDSO numerical-quadrature pricer.
//!
//! Reference: Bloomberg L.P. Quantitative Analytics, "Pricing Credit Index
//! Options" (DOCS 2055833 ⟨GO⟩, March 2012). The Bloomberg CDS model is
//! described in DOCS 2057273 ⟨GO⟩ (August 2024).
//!
//! # Model
//!
//! Implements the Bloomberg CDSO model
//!
//! ```text
//! O = P(t_e) · E_0[(V_te + H(K) + D)+]
//! ```
//!
//! where:
//!
//! - `V_te = ξN(S_te − c) · L_te(S_te)` is the underlying swap value at the
//!   option expiry. `L_te(S)` is the *forward* clean risky annuity computed
//!   under a flat hazard rate `λ = S/(1−R)` over `[t_e, t_M]`.
//! - `H(K) = ξN(c − K) · A(K)` is the **strike-adjustment** term — a
//!   contractual cash flow upon exercise to compensate for the difference
//!   between the option strike `K` and the contractual coupon `c` of the
//!   underlying swap. `A(K)` is the deterministic forward annuity at flat
//!   spread `K`.
//! - `D = ξN_0 · loss(t_v)` is the **loss-settlement** term, accounting for
//!   defaults that have settled between option inception and the valuation
//!   date `t_v`. Currently 0 for all in-house fixtures (no historical
//!   defaults on the test indices).
//! - `S_te` is lognormally distributed under the risk-neutral measure with
//!   `σ` the user-supplied spread vol and mean `m` calibrated so that
//!   `F_0 = E_0[V_te]` (the no-knockout clean forward value).
//! - `ξ = +1` for payer (call), `ξ = −1` for receiver (put).
//!
//! # Simplifications (per Bloomberg paper §2.5)
//!
//! Bloomberg's published implementation makes two simplifying assumptions:
//!
//! 1. The interest rate up to swap expiry is constant.
//! 2. The index coupon is paid continuously (rather than quarterly).
//!
//! These yield an analytic relationship `λ(S) = S/(1−R)` between spread and
//! hazard rate (the "credit triangle"), eliminating per-quadrature-node
//! root finding for `λ`. We follow the same simplifications, evaluating
//! the forward annuity per quadrature node analytically.
//!
//! # Quadrature scheme
//!
//! A trapezoidal rule on the standard normal density is used over `z ∈
//! [−6, 6]` with step `dz = 0.05`, matching the FinancePy reference
//! (`CDSIndexOption._calc_index_payer_option_price`) but with finer steps.
//! At each quadrature node `z`,
//!
//! ```text
//! S_te = m · exp(−½σ²t_e + σ·√t_e · z)
//! V_te / N = (S_te − c) · L_te(S_te)
//! ```
//!
//! # Calibration of `m`
//!
//! The lognormal mean `m` is solved via Brent root-finding so that the
//! risk-neutral expectation of the swap value matches the no-knockout
//! forward value seen at `t_v`. The no-knockout forward is
//!
//! ```text
//! F_0/N = (1−R)·(1−q_te)  +  (s_par − c) · A(s_par)
//! ```
//!
//! where `s_par` is the bootstrapped forward CDS par spread and `q_te` is
//! the index survival probability to expiry on the bootstrapped curve.
//! The first term is the FEP-equivalent contribution; the second is the
//! deterministic value of receiving `(s_par − c)` of clean coupons over
//! the forward annuity.
//!
//! # Greeks
//!
//! The current implementation returns NPV only. Greeks (Δ, Γ, Vega, Theta)
//! continue to flow through the legacy Black formulation pending a follow-up
//! that propagates the quadrature model end-to-end. Bloomberg's IR DV01 and
//! Spread DV01 use parallel curve bumps and reprice through the same
//! quadrature, so the existing `*Cs01Calculator`/`*Dv01Calculator` paths
//! (which already bump and reprice via `npv()`) inherit the new model
//! automatically once it is wired in.

use crate::constants::{numerical, BASIS_POINTS_PER_UNIT};
use crate::instruments::common_impl::parameters::OptionType;
use crate::instruments::credit_derivatives::cds::pricer::CDSPricer;
use crate::instruments::credit_derivatives::cds::CreditDefaultSwap;
use crate::instruments::credit_derivatives::cds_option::CDSOption;
use finstack_core::dates::Date;
use finstack_core::market_data::context::MarketContext;
use finstack_core::market_data::term_structures::{DiscountCurve, HazardCurve};
use finstack_core::math::solver::{BrentSolver, Solver};
use finstack_core::money::Money;
use finstack_core::Result;
use rust_decimal::prelude::ToPrimitive;
use rust_decimal::Decimal;

const G_DAYS_IN_YEAR: f64 = 365.0;

/// Standard-normal quadrature parameters. `Z_LIMIT` covers ~6σ on each side
/// of the mean (the lognormal density is effectively zero outside this
/// range for typical CDS option vols), and `Z_STEP` is the trapezoidal
/// integration step.
const Z_LIMIT: f64 = 6.0;
const Z_STEP: f64 = 0.05;

/// Inverse of √(2π). Multiplies the trapezoidal sum to produce the
/// risk-neutral expectation under the standard normal density.
const INV_ROOT_2_PI: f64 = 0.398_942_280_401_432_7_f64;

/// Pre-computed deterministic inputs to the Bloomberg quadrature.
/// Built once at the top of `npv()` so the quadrature loop and the
/// calibration loop share the same schedule and curves.
struct ForwardCdsContext {
    /// Year-fractions from option expiry to each post-expiry coupon
    /// payment date, in calendar days / 365 (matches FinancePy).
    times_from_expiry: Vec<f64>,
    /// Premium-leg accrual factors per coupon period in the synthetic
    /// CDS day-count (typically Act/360 for USD CDX). One entry per
    /// post-expiry payment.
    accrual_factors: Vec<f64>,
    /// Forward discount factors from `t_e` to each payment date,
    /// `df(t_pay) / df(t_e)`. One entry per post-expiry payment.
    fwd_discount_factors: Vec<f64>,
    /// Year-fraction from the previous coupon date (PCD) to `t_e`,
    /// in the synthetic CDS day-count. Subtracts the accrued portion
    /// to convert dirty RPV01 → clean RPV01.
    accrual_pcd_to_expiry: f64,
    /// `1 − R` on the synthetic underlying.
    lgd: f64,
    /// `df(t_e)`: discount factor from valuation date to expiry.
    df_to_expiry: f64,
    /// Survival to expiry on the bootstrapped index curve.
    survival_to_expiry: f64,
    /// Forward par spread `s_par` from the bootstrapped curve (decimal,
    /// i.e. 0.005528 for 55.28 bp).
    forward_par_spread: f64,
    /// Bootstrapped forward clean RPV01 expressed *at expiry*
    /// (i.e. `clean_rpv01_from_value_dt / (df_te · q_te)`). Used in
    /// the calibration target `F_0`.
    bootstrapped_l_at_expiry: f64,
    /// Index contractual coupon `c` (decimal).
    coupon: f64,
    /// Option strike `K` (decimal).
    strike: f64,
    /// Option time-to-expiry `t_e` in years (calendar days / 365).
    t_expiry: f64,
    /// Lognormal spread volatility `σ`.
    sigma: f64,
    /// True for CDS-index options (CDX, iTraxx) — these trade on a
    /// no-knockout basis, so the Bloomberg `F_0` calibration target
    /// includes the `(1−R)·(1−q_te)` FEP-equivalent contribution. For
    /// single-name CDS options the calibration drops this term because
    /// the option knocks out on default of the reference entity.
    is_index: bool,
}

impl ForwardCdsContext {
    fn build(
        option: &CDSOption,
        disc: &DiscountCurve,
        surv: &HazardCurve,
        cds: &CreditDefaultSwap,
        as_of: Date,
        sigma: f64,
    ) -> Result<Self> {
        let cds_pricer = CDSPricer::new();
        let lgd = (1.0 - option.recovery_rate).max(numerical::ZERO_TOLERANCE);

        // Continuous time-to-expiry per Bloomberg paper §2.5 (calendar days / 365).
        let t_expiry = ((option.expiry - as_of).whole_days() as f64) / G_DAYS_IN_YEAR;

        // df(t_e) using the discount curve's date-based interface.
        let df_to_expiry = DiscountCurve::df_between_dates(disc, as_of, option.expiry)?;

        // Survival to expiry (bootstrapped curve) — used in the FEP-like
        // term of the no-knockout forward.
        let survival_to_expiry = surv.sp_on_date(option.expiry).unwrap_or(1.0).clamp(0.0, 1.0);

        // Premium-leg cashflows of the synthetic forward CDS, restricted
        // to post-expiry payments. We use the same `coupon_periods` walker
        // that the existing pricer uses, so the schedule is exactly the
        // forward annuity Bloomberg integrates over.
        let cashflows = cds_pricer.premium_cashflow_accruals(cds, as_of)?;
        let mut times_from_expiry = Vec::with_capacity(cashflows.len());
        let mut accrual_factors = Vec::with_capacity(cashflows.len());
        let mut fwd_discount_factors = Vec::with_capacity(cashflows.len());
        for (pay_date, accrual) in cashflows.iter() {
            if *pay_date <= option.expiry {
                continue;
            }
            let t_e_to_pay = ((*pay_date - option.expiry).whole_days() as f64) / G_DAYS_IN_YEAR;
            let df_pay = DiscountCurve::df_between_dates(disc, as_of, *pay_date)?;
            let fwd_df = if df_to_expiry > numerical::ZERO_TOLERANCE {
                df_pay / df_to_expiry
            } else {
                0.0
            };
            times_from_expiry.push(t_e_to_pay);
            accrual_factors.push(*accrual);
            fwd_discount_factors.push(fwd_df);
        }

        // Year-fraction from the previous coupon date (PCD ≤ expiry) to
        // expiry, in the CDS day-count. PCD is the latest coupon-period
        // accrual_start that is ≤ expiry; when no such date exists (i.e.
        // the synthetic forward CDS premium starts strictly after expiry,
        // as in the standard CDX CDSO setup), `accrual_pcd_to_expiry = 0`
        // — there is no pre-expiry accrued portion to net out.
        let mut pcd: Option<Date> = None;
        for (pay_date, _) in cashflows.iter() {
            // payment_date == accrual_end for BloombergCdswClean. We want the
            // latest accrual_start ≤ expiry; use the prior payment_date as a
            // proxy for the current period's accrual_start.
            if *pay_date <= option.expiry {
                pcd = Some(*pay_date);
            } else {
                break;
            }
        }
        let pcd = pcd.unwrap_or(cds.premium.start.min(option.expiry));
        let accrual_pcd_to_expiry = if pcd >= option.expiry {
            0.0
        } else {
            finstack_core::dates::DayCount::year_fraction(
                cds.premium.day_count,
                pcd,
                option.expiry,
                finstack_core::dates::DayCountContext::default(),
            )
            .unwrap_or(0.0)
        };

        // Bootstrapped forward par spread and forward clean RPV01 — the
        // latter is what Bloomberg calls L_te in the no-knockout forward
        // value, expressed *at expiry* (so we divide by df_te · q_te).
        let premium_per_bp =
            cds_pricer.forward_premium_leg_pv_per_bp(cds, disc, surv, as_of, option.expiry)?;
        let risky_annuity_at_value_dt =
            premium_per_bp / crate::constants::ONE_BASIS_POINT;
        let denom = (df_to_expiry * survival_to_expiry).max(numerical::ZERO_TOLERANCE);
        let bootstrapped_l_at_expiry = risky_annuity_at_value_dt / denom;

        let forward_protection_pv = cds_pricer
            .pv_protection_leg(cds, disc, surv, as_of)?
            .amount();
        let denom_par =
            (risky_annuity_at_value_dt * cds.notional.amount()).max(numerical::ZERO_TOLERANCE);
        let natural_forward_par_spread = forward_protection_pv / denom_par;
        // The fixture-level `forward_spread_adjust` calibrates finstack's
        // bootstrapped forward par to the Bloomberg-quoted ATM Forward (the
        // residual convention gap between the two implementations' bootstrap
        // routines). Apply it here so the calibration target `F_0 = E[V_te]`
        // is aligned with the Bloomberg screen value, not finstack's
        // independent bootstrap result.
        let adjust = decimal_to_f64(option.forward_spread_adjust)?;
        let forward_par_spread = natural_forward_par_spread + adjust;

        let coupon = decimal_to_f64(option.effective_underlying_cds_coupon())?;
        let strike = decimal_to_f64(option.strike)?;

        Ok(Self {
            times_from_expiry,
            accrual_factors,
            fwd_discount_factors,
            accrual_pcd_to_expiry,
            lgd,
            df_to_expiry,
            survival_to_expiry,
            forward_par_spread,
            bootstrapped_l_at_expiry,
            coupon,
            strike,
            t_expiry,
            sigma,
            is_index: option.underlying_is_index,
        })
    }

    /// Forward clean RPV01 *at expiry* under a flat hazard `λ = s/(1−R)`,
    /// over the synthetic premium-leg schedule from expiry to maturity.
    ///
    /// Equivalent to FinancePy's inner loop in
    /// `_calc_index_payer_option_price`:
    ///
    /// ```text
    /// fwd_rpv01 = Σ_i acc_i · exp(−λ · t_i_from_expiry) · fwd_df_i
    ///           − accrual_factor_pcd_to_expiry
    /// ```
    fn flat_annuity(&self, s: f64) -> f64 {
        let lambda = s / self.lgd;
        let mut acc = -self.accrual_pcd_to_expiry;
        for ((acc_factor, t), fwd_df) in self
            .accrual_factors
            .iter()
            .zip(self.times_from_expiry.iter())
            .zip(self.fwd_discount_factors.iter())
        {
            let surv = (-lambda * t).exp();
            acc += acc_factor * surv * fwd_df;
        }
        acc
    }

    /// Per-unit-notional swap value at expiry under flat-spread `s`:
    /// `(s − c) · L(s)`.
    fn swap_value_per_n(&self, s: f64) -> f64 {
        (s - self.coupon) * self.flat_annuity(s)
    }

    /// No-knockout forward value (per unit notional) — the calibration
    /// target for the lognormal mean `m`. Combines the FEP-like
    /// default-protection contribution `(1−R)(1−q_te)` with the clean
    /// forward swap value at the bootstrapped par spread, per Bloomberg
    /// CDSO §2.5: *"the no-knockout forward price ... is calculated
    /// exactly according to market conventions"* — i.e. exact bootstrap,
    /// not the flat-spread approximation that the quadrature integrand
    /// uses internally. The two sides being intentionally different is
    /// the design of the model.
    fn no_knockout_forward(&self) -> f64 {
        // FEP-equivalent default-protection contribution. Only applies to
        // index options that trade on a no-knockout basis (Bloomberg paper
        // § 1.2: *"the forward index spread to the option expiration date
        // is computed as the replacement spread on a forward starting swap
        // with no-knockout to account for defaults taking place between
        // the valuation date and option expiry"*). Single-name CDS options
        // knock out on default per standard market convention, so this
        // term is omitted for single-name underlyings.
        let h1 = if self.is_index {
            self.lgd * (1.0 - self.survival_to_expiry)
        } else {
            0.0
        };
        let h2 = (self.forward_par_spread - self.coupon) * self.bootstrapped_l_at_expiry;
        h1 + h2
    }
}

/// Calibrate the lognormal mean `m` so that the risk-neutral expectation
/// `E[V_te(S_te(m))]` matches the no-knockout forward value.
///
/// `S_te(m, ε) = m · exp(−½σ²t + σ√t·ε)` for `ε ~ N(0, 1)`. The expected
/// swap value is monotonic in `m` for typical inputs, so a Brent root find
/// converges in a handful of iterations.
fn calibrate_m(ctx: &ForwardCdsContext) -> Result<f64> {
    let target = ctx.no_knockout_forward();
    let s0 = (-0.5 * ctx.sigma * ctx.sigma * ctx.t_expiry).exp();
    let sigma_sqrt_t = ctx.sigma * ctx.t_expiry.sqrt();

    let expected_swap_value = |m: f64| -> f64 {
        let mut acc = 0.0;
        let mut z = -Z_LIMIT;
        let n_steps = ((2.0 * Z_LIMIT) / Z_STEP).round() as usize;
        for _ in 0..n_steps {
            let s = m * s0 * (sigma_sqrt_t * z).exp();
            let v = ctx.swap_value_per_n(s);
            let pdf = (-0.5 * z * z).exp();
            acc += v * pdf;
            z += Z_STEP;
        }
        acc * INV_ROOT_2_PI * Z_STEP
    };

    // Solve `g(log m) = E[V(m)] − target = 0` in log-space so the
    // unbounded m > 0 search is well-conditioned.
    let f = |log_m: f64| -> f64 { expected_swap_value(log_m.exp()) - target };

    // Initial guess: the bootstrapped forward par spread is the natural
    // starting point — for an at-the-money option the calibrated m is
    // close to par.
    let x0 = ctx.forward_par_spread.max(1e-6).ln();
    let solver = BrentSolver::new().tolerance(1e-12);
    let log_m = solver.solve(f, x0)?;
    Ok(log_m.exp())
}

/// Evaluate the option premium (per unit notional) by quadrature over the
/// standard normal density.
fn quadrature_payoff(ctx: &ForwardCdsContext, m: f64, scale: f64, sign: f64, h_k: f64, d: f64) -> f64 {
    let s0 = (-0.5 * ctx.sigma * ctx.sigma * ctx.t_expiry).exp();
    let sigma_sqrt_t = ctx.sigma * ctx.t_expiry.sqrt();

    let mut acc = 0.0;
    let mut z = -Z_LIMIT;
    let n_steps = ((2.0 * Z_LIMIT) / Z_STEP).round() as usize;
    for _ in 0..n_steps {
        let s = m * s0 * (sigma_sqrt_t * z).exp();
        let v = ctx.swap_value_per_n(s); // V_te per N
        // Payoff per unit notional with payer/receiver sign, including
        // strike adjustment H(K) and loss-settlement D.
        let payoff = (sign * v + h_k + d).max(0.0);
        let pdf = (-0.5 * z * z).exp();
        acc += payoff * pdf;
        z += Z_STEP;
    }
    scale * acc * INV_ROOT_2_PI * Z_STEP * ctx.df_to_expiry
}

/// Public entry point: price a CDS option under the Bloomberg CDSO model.
pub(crate) fn npv(
    option: &CDSOption,
    cds: &CreditDefaultSwap,
    curves: &MarketContext,
    sigma: f64,
    as_of: Date,
) -> Result<Money> {
    option.validate_supported_configuration()?;

    let disc = curves.get_discount(&option.discount_curve_id)?;
    let surv = curves.get_hazard(&option.credit_curve_id)?;

    let ctx = ForwardCdsContext::build(option, disc.as_ref(), surv.as_ref(), cds, as_of, sigma)?;

    // Expired or zero-life options collapse to intrinsic on a degenerate
    // forward. We mirror the legacy pricer's behaviour and let downstream
    // code handle the t ≤ 0 case explicitly.
    if ctx.t_expiry <= 0.0 || sigma <= 0.0 {
        let intrinsic = match option.option_type {
            OptionType::Call => (ctx.forward_par_spread - ctx.strike).max(0.0),
            OptionType::Put => (ctx.strike - ctx.forward_par_spread).max(0.0),
        };
        let scale = if option.underlying_is_index {
            option.index_factor.unwrap_or(1.0)
        } else {
            1.0
        };
        return Ok(Money::new(
            scale * intrinsic * ctx.bootstrapped_l_at_expiry * option.notional.amount(),
            option.notional.currency(),
        ));
    }

    // Calibrate the lognormal mean to the no-knockout forward value.
    let m = calibrate_m(&ctx)?;

    // Strike adjustment H(K) per unit notional. ξ is +1 for payer, −1 for
    // receiver. Inside the max(...) below we apply ξ to V_te; H(K) is
    // taken with sign ξ as well (per Bloomberg paper §2.2).
    let sign = match option.option_type {
        OptionType::Call => 1.0,
        OptionType::Put => -1.0,
    };
    let a_k = ctx.flat_annuity(ctx.strike);
    let h_k = sign * (ctx.coupon - ctx.strike) * a_k;

    // Loss settlement D is currently zero for in-house fixtures; reserved
    // here for future support of saved deals with realized index losses.
    let d = 0.0;

    let scale = if option.underlying_is_index {
        option.index_factor.unwrap_or(1.0)
    } else {
        1.0
    };

    let pv_per_n = quadrature_payoff(&ctx, m, scale, sign, h_k, d);
    Ok(Money::new(
        pv_per_n * option.notional.amount(),
        option.notional.currency(),
    ))
}

/// Black-formula forward spread (decimal) and risky annuity (year-fractions
/// at expiry) — exposed so the par_spread metric can keep its current
/// contract: report the *bootstrapped* forward par spread, not the
/// quadrature's calibrated mean. Bloomberg's CDSO screen quotes ATM Fwd
/// as the par spread of the no-knockout forward CDS, and that is exactly
/// `ctx.forward_par_spread` in this module.
pub(crate) fn forward_par_and_annuity(
    option: &CDSOption,
    cds: &CreditDefaultSwap,
    curves: &MarketContext,
    as_of: Date,
) -> Result<(f64, f64)> {
    let disc = curves.get_discount(&option.discount_curve_id)?;
    let surv = curves.get_hazard(&option.credit_curve_id)?;
    let ctx =
        ForwardCdsContext::build(option, disc.as_ref(), surv.as_ref(), cds, as_of, 0.0)?;
    Ok((
        ctx.forward_par_spread * BASIS_POINTS_PER_UNIT,
        ctx.bootstrapped_l_at_expiry,
    ))
}

fn decimal_to_f64(value: Decimal) -> Result<f64> {
    value.to_f64().ok_or_else(|| {
        finstack_core::Error::Validation(format!(
            "CDS option Bloomberg quadrature: cannot convert {value} to f64"
        ))
    })
}
