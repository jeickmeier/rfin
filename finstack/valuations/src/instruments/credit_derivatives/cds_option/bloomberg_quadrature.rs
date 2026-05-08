//! Bloomberg CDSO numerical-quadrature pricer.
//!
//! Implements the model published in:
//!
//! - Bloomberg L.P. Quantitative Analytics. *Pricing Credit Index Options.*
//!   DOCS 2055833 ⟨GO⟩, March 2012.
//!
//! and uses the Bloomberg CDS pricer (DOCS 2057273) for the bootstrapped
//! `F_0` calibration target.
//!
//! # Model summary (DOCS 2055833 §2.2, Eqs. 2.2–2.5)
//!
//! ```text
//! S_te    = m · exp(−½σ²t_e + σ·√t_e · ε),     ε ~ N(0,1)
//! V_te    = ξ N (S_te − c) · L_te(S_te)
//! H(K)    = ξ N (c − K) · A(K)
//! D       = ξ N₀ · loss(t_v)
//! O       = P(t_e) · E_0 [ (V_te + H(K) + D)+ ]
//! F_0     = E_0 [V_te]                          (calibration anchor)
//! ```
//!
//! - `S_te` is the (random) realised forward CDS spread at expiry.
//! - `L_te(S)` is the *flat-spread* forward risky annuity at hazard
//!   `λ = S/(1−R)` over `[t_e, t_M]` — the "credit triangle" simplification
//!   §2.5: continuous coupon, constant rate to expiry → analytic `λ(S)`.
//! - `A(K)` is the same flat-spread annuity evaluated at `S = K`.
//! - `m` is calibrated so the no-knockout forward `F_0` matches the
//!   bootstrapped clean forward swap value.
//! - `ξ = +1` for payer (call), `ξ = −1` for receiver (put).
//! - For single-name CDS options the `(1−R)·(1−q_te)` FEP-equivalent term
//!   in `F_0` is omitted (single-name options knock out on default).
//!
//! # Numerical integration
//!
//! Trapezoidal rule on the standard normal density over `z ∈ [−6, 6]` with
//! step `Δz = 0.05`. The integrand is smooth (lognormal × piecewise-linear
//! in `(s−c)L(s)`) so 240 quadrature nodes give 1e-9 absolute precision —
//! well below the precision the calibration achieves.
//!
//! All time inputs use **calendar days / 365** (DOCS 2055833 §2.1, matching
//! FinancePy's `G_DAYS_IN_YEAR = 365.0`). Premium-leg accrual factors come
//! from the synthetic underlying CDS in its native day count (Act/360 for
//! USD CDX/iTraxx Main).

use crate::constants::{numerical, BASIS_POINTS_PER_UNIT};
use crate::instruments::common_impl::parameters::OptionType;
use crate::instruments::credit_derivatives::cds::pricer::CDSPricer;
use crate::instruments::credit_derivatives::cds::CreditDefaultSwap;
use crate::instruments::credit_derivatives::cds_option::CDSOption;
use finstack_core::dates::Date;
use finstack_core::market_data::context::MarketContext;
use finstack_core::market_data::term_structures::{DiscountCurve, HazardCurve};
use finstack_core::math::solver::BrentSolver;
use finstack_core::money::Money;
use finstack_core::Result;
use rust_decimal::prelude::ToPrimitive;
use rust_decimal::Decimal;

/// Bloomberg CDSO calendar-day denominator (DOCS 2055833 §2.1).
const G_DAYS_IN_YEAR: f64 = 365.0;

/// Bloomberg CDSO theta day basis (DOCS 2055833 §2.5: *"shortening the
/// exercise time `t_e` by `1/365.25`"*).
#[allow(dead_code)]
const THETA_DAYS_IN_YEAR: f64 = 365.25;

/// Standard-normal quadrature: integrate over `z ∈ [−Z_LIMIT, +Z_LIMIT]`
/// in steps of `Z_STEP`. Six standard deviations cover the lognormal
/// support to ~2e-9 cumulative tail probability.
const Z_LIMIT: f64 = 6.0;
const Z_STEP: f64 = 0.05;

/// `1/√(2π)` — the standard normal density's normalising constant.
const INV_SQRT_2_PI: f64 = 0.398_942_280_401_432_7_f64;

// =====================================================================
// Public entry points
// =====================================================================

/// Price a CDS option under the Bloomberg CDSO numerical-quadrature model.
pub(crate) fn npv(
    option: &CDSOption,
    cds: &CreditDefaultSwap,
    curves: &MarketContext,
    sigma: f64,
    as_of: Date,
) -> Result<Money> {
    let disc = curves.get_discount(&option.discount_curve_id)?;
    let surv = curves.get_hazard(&option.credit_curve_id)?;

    let ctx = ForwardCdsContext::build(option, disc.as_ref(), surv.as_ref(), cds, as_of, sigma)?;

    // Eq. 2.3: solve `m` so E[V_te(S_te(m))] matches the no-knockout F_0.
    let m = calibrate_lognormal_mean(&ctx)?;

    // Eq. 2.5: O = P(t_e) · E_0[(ξV_te + H(K) + D)+]
    let pv_per_n = price_with_calibrated_mean(&ctx, m, ctx.t_expiry.max(0.0));
    Ok(Money::new(
        pv_per_n * option.notional.amount(),
        option.notional.currency(),
    ))
}

/// Bloomberg CDSO theta: shorten the exercise time by 1/365.25 while
/// retaining the same calibrated forward price and lognormal mean.
pub(crate) fn theta(
    option: &CDSOption,
    cds: &CreditDefaultSwap,
    curves: &MarketContext,
    sigma: f64,
    as_of: Date,
) -> Result<f64> {
    let disc = curves.get_discount(&option.discount_curve_id)?;
    let surv = curves.get_hazard(&option.credit_curve_id)?;
    let ctx = ForwardCdsContext::build(option, disc.as_ref(), surv.as_ref(), cds, as_of, sigma)?;
    if ctx.t_expiry <= 0.0 {
        return Ok(0.0);
    }
    let m = calibrate_lognormal_mean(&ctx)?;
    let base = price_with_calibrated_mean(&ctx, m, ctx.t_expiry);
    let shortened_t = (ctx.t_expiry - (1.0 / THETA_DAYS_IN_YEAR)).max(0.0);
    let bumped = price_with_calibrated_mean(&ctx, m, shortened_t);
    Ok((bumped - base) * option.notional.amount())
}

/// Bloomberg CDSO ATM Forward (in basis points) — the bootstrapped forward
/// par spread of the no-knockout forward CDS at expiry.
pub(crate) fn forward_par_at_expiry_bp(
    option: &CDSOption,
    cds: &CreditDefaultSwap,
    curves: &MarketContext,
    as_of: Date,
) -> Result<f64> {
    let disc = curves.get_discount(&option.discount_curve_id)?;
    let surv = curves.get_hazard(&option.credit_curve_id)?;
    let ctx = ForwardCdsContext::build(option, disc.as_ref(), surv.as_ref(), cds, as_of, 0.0)?;
    Ok(ctx.display_forward_par_spread * BASIS_POINTS_PER_UNIT)
}

// =====================================================================
// Pre-computed deterministic inputs
// =====================================================================

/// All deterministic quantities the quadrature integrand and the
/// calibration target need. Built once at the top of `npv()` from the
/// instrument + curves; the calibration loop and the payoff loop both
/// borrow it.
struct ForwardCdsContext {
    /// `1 − R` on the synthetic underlying.
    lgd: f64,
    /// `t_e` in years (`(expiry − as_of)/365`).
    t_expiry: f64,
    /// `σ` from the vol surface or instrument override.
    sigma: f64,
    /// `df(t_e)` from valuation date to expiry, on the option's discount
    /// curve.
    df_to_expiry: f64,
    /// Conditional survival probability from valuation date to expiry on
    /// the bootstrapped credit curve.
    survival_to_expiry: f64,
    /// Bootstrapped forward par spread `s_par` (decimal), computed using
    /// the PCD-corrected annuity. Used as the calibration anchor for the
    /// no-knockout forward F_0 = h1 + (par − c) · L_te (DOCS 2055833 §2.3).
    forward_par_spread: f64,
    /// Bloomberg HELP CDSO "ATM Fwd" display value (decimal): same
    /// `spot_protection_pv` numerator, but with the Bloomberg-screen
    /// drop-first-cashflow annuity in the denominator. Reported to the
    /// metrics framework via [`forward_par_at_expiry_bp`] but NOT used in
    /// the option NPV calibration.
    display_forward_par_spread: f64,
    /// Bootstrapped clean RPV01 of the forward CDS *expressed at expiry*
    /// (i.e. divided by `df_te · q_te`). Used in the F_0 calibration target.
    bootstrapped_l_at_expiry: f64,
    /// Year fractions from `t_e` to each post-expiry coupon payment date.
    /// `(payment_date − expiry) / 365`.
    times_from_expiry: Vec<f64>,
    /// Premium-leg accrual factors per coupon period in the synthetic
    /// CDS day-count (typically Act/360). One entry per post-expiry
    /// coupon payment.
    accrual_factors: Vec<f64>,
    /// Forward discount factors `df(t_pay) / df(t_e)`. One entry per
    /// post-expiry payment.
    fwd_discount_factors: Vec<f64>,
    /// Year fraction from the previous coupon date `T_{n(t_e)}` to `t_e`,
    /// in the synthetic CDS day-count. Subtracted from the dirty per-bp
    /// annuity to convert to clean. For forward CDSes whose premium starts
    /// at expiry this is zero.
    accrual_pcd_to_expiry: f64,
    /// Index contractual coupon `c` (decimal).
    coupon: f64,
    /// Option strike `K` (decimal).
    strike: f64,
    /// `ξ = +1` for payer (Call), `−1` for receiver (Put).
    option_type: OptionType,
    /// Index-factor scale (1.0 for non-index or original-version index
    /// underlyings).
    scale: f64,
    /// Realized index loss per unit of original notional.
    realized_index_loss: f64,
    /// True for index options (no-knockout calibration uses the FEP-like
    /// term in F_0).
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

        let t_expiry = ((option.expiry - as_of).whole_days() as f64) / G_DAYS_IN_YEAR;
        let df_to_expiry = DiscountCurve::df_between_dates(disc, as_of, option.expiry)?;
        let sp_asof = surv
            .sp_on_date(as_of)
            .unwrap_or(1.0)
            .clamp(numerical::ZERO_TOLERANCE, 1.0);
        let sp_expiry = surv
            .sp_on_date(option.expiry)
            .unwrap_or(1.0)
            .clamp(0.0, 1.0);
        let survival_to_expiry = (sp_expiry / sp_asof).clamp(0.0, 1.0);

        // Premium-leg coupon schedule of the synthetic forward CDS,
        // restricted to payments strictly after expiry. We also build the
        // no-AoD post-expiry risky-annuity sum directly here — Bloomberg's
        // ATM Fwd "Premium Leg" (HELP CDSO) and the ISDA-standard
        // `risky_annuity` denominator (DOCS 2057273 Eq 3.3) are coupons-in-
        // survival only, *without* the accrual-on-default integral. Using
        // A forward premium-leg PV that includes AoD would systematically
        // overstate the annuity by the AoD contribution, which on cdx_ig_46
        // shifts ATM Fwd by ~0.1 bp.
        let cashflows = cds_pricer.premium_cashflow_accruals(cds, as_of)?;
        let mut times_from_expiry = Vec::with_capacity(cashflows.len());
        let mut accrual_factors = Vec::with_capacity(cashflows.len());
        let mut fwd_discount_factors = Vec::with_capacity(cashflows.len());
        let mut raw_annuity_at_value_dt_no_aod = 0.0_f64;
        let mut first_post_expiry_pv01_at_value_dt = 0.0_f64;
        let mut seen_first_post_expiry = false;
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
            let sp_pay_uncond = surv.sp_on_date(*pay_date).unwrap_or(1.0).clamp(0.0, 1.0);
            let sp_pay_cond = (sp_pay_uncond / sp_asof).clamp(0.0, 1.0);
            times_from_expiry.push(t_e_to_pay);
            accrual_factors.push(*accrual);
            fwd_discount_factors.push(fwd_df);
            let cf_pv01 = *accrual * df_pay * sp_pay_cond;
            raw_annuity_at_value_dt_no_aod += cf_pv01;
            if !seen_first_post_expiry {
                first_post_expiry_pv01_at_value_dt = cf_pv01;
                seen_first_post_expiry = true;
            }
        }

        // Pre-expiry "previous coupon date" → expiry year-fraction. For a
        // forward CDS whose premium starts at expiry this is zero.
        let mut pcd: Option<Date> = None;
        for (pay_date, _) in cashflows.iter() {
            if *pay_date <= option.expiry {
                pcd = Some(*pay_date);
            } else {
                break;
            }
        }
        let pcd = pcd.unwrap_or_else(|| cds.premium.start.min(option.expiry));
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

        // Bootstrapped forward par spread and clean forward RPV01 — the
        // Bloomberg CDSO "ATM Fwd" computation, per the published Help
        // methodology (HELP CDSO <GO> "Calculating ATM Forward Spread for
        // CDSO"):
        //
        //   ATM Fwd = Default_Leg(0, T_mat) / Premium_Leg(T_exp, T_mat)
        //
        //   Default Leg: PV of expected loss from the **valuation date** to
        //   the underlying CDS maturity — i.e., the *spot* protection PV.
        //   Premium Leg: PV of a 1bp premium stream from [T_exp + 1, T_mat]
        //   on the underlying CDS schedule, **subtracting the PV01 of the
        //   first cashflow** (Bloomberg's verbatim wording — the first
        //   post-expiry coupon, i.e. the one whose accrual period straddles
        //   T_exp, is dropped in full).
        //
        // A plain post-expiry premium-leg sum includes the first coupon in full;
        // `pv_protection_leg` integrates from `max(as_of, protection_start)`
        // to maturity — when the synthetic CDS has `premium.start ≤ as_of`
        // and `protection_effective_date = None` (the spot configuration set
        // up by `synthetic_underlying_cds`), this is exactly the spot
        // Default_Leg(0, T_mat) Bloomberg's formula calls for.
        let denom_te = (df_to_expiry * survival_to_expiry).max(numerical::ZERO_TOLERANCE);
        // Bloomberg HELP CDSO ATM Fwd: "subtract the PV01 of the first
        // cashflow." Apply this rule only when there is a STRADDLING
        // first period (premium.start strictly before T_exp), in which
        // case dropping the full first cashflow PV01 is the BBG screen
        // formula. When `premium.start ≥ T_exp` (no straddle), the first
        // post-expiry cashflow is a stub starting at premium.start with
        // no pre-expiry component to net out, and dropping it would
        // distort the option's internal forward vs. the standard CDS
        // par_spread of the same underlying — so we leave the annuity
        // unchanged in that case (matching the legacy PCD behaviour
        // that the put/call-parity-at-forward invariant relies on).
        // For the calibration target F_0 = (par − c) · L_te and the
        // quadrature integrand V_te(s) = (s − c) · L_te(s), we use the
        // economically-meaningful "PCD subtraction" — the pre-expiry
        // portion of the period straddling expiry. This preserves
        // put/call parity at ATF for forward CDSes whose schedule starts
        // at T_exp (no straddle ⇒ no subtraction), and gives a
        // self-consistent calibration.
        let pcd_stub_at_value_dt = accrual_pcd_to_expiry * denom_te;
        let risky_annuity_at_value_dt = raw_annuity_at_value_dt_no_aod - pcd_stub_at_value_dt;
        let bootstrapped_l_at_expiry = risky_annuity_at_value_dt / denom_te;

        // Bloomberg HELP CDSO ATM Fwd display formula: "Premium Leg = PV
        // of 1bp stream from [T_exp+1, T_mat], subtracting the PV01 of
        // the first cashflow." When the synthetic CDS schedule has a
        // straddling first period (premium.start strictly < T_exp), this
        // is the literal full-first-cashflow drop. When there's no
        // straddle, the Bloomberg formula degenerates to the standard
        // post-expiry sum (the "first cashflow" is then a thin stub
        // already starting at T_exp). The displayed `forward_par_spread`
        // uses this annuity; the calibration/integrand use the PCD
        // version above. Decoupling display from math is necessary
        // because the drop-first formula moves par by ~0.16 bp on
        // cdx_ig_46 while the calibration would overshoot if it tracked
        // the same shift.
        let drop_first_cashflow = cds.premium.start < option.expiry;
        let display_annuity_at_value_dt = if drop_first_cashflow {
            raw_annuity_at_value_dt_no_aod - first_post_expiry_pv01_at_value_dt
        } else {
            risky_annuity_at_value_dt
        };

        // Bloomberg DOCS 2057273 §3 protection-leg convention:
        // "Protection starts immediately, therefore the full number of days
        // for protection and coupon is (TM − T + 1)." We honour the +1-day
        // inclusive end of protection here (scoped to the option pricer) by
        // building a temporary CDS with `premium.end + 1 day` and computing
        // protection on that. We don't change `pv_protection_leg` globally
        // because non-forward-CDS pricing in finstack assumes the standard
        // [T, TM] integration; the +1-day rule is a CDSO-specific tightening
        // that closes ~0.05 bp of the cdx_ig_46 ATM Fwd residual.
        let mut spot_cds_plus_one = cds.clone();
        spot_cds_plus_one.premium.end = cds.premium.end + time::Duration::days(1);
        let spot_protection_pv = cds_pricer
            .pv_protection_leg(&spot_cds_plus_one, disc, surv, as_of)?
            .amount();
        // ECONOMIC forward par — used for calibration target F_0 = h1 + (par − c) · L_te.
        // Uses the same PCD-corrected annuity as `bootstrapped_l_at_expiry` so the
        // calibration is internally self-consistent.
        let economic_denom_par =
            (risky_annuity_at_value_dt * cds.notional.amount()).max(numerical::ZERO_TOLERANCE);
        let forward_par_spread = spot_protection_pv / economic_denom_par;
        // DISPLAY-ONLY par (Bloomberg HELP CDSO ATM Fwd formula). Reported via
        // `forward_par_at_expiry_bp` for the par_spread metric. Differs from
        // the economic par when the synthetic CDS schedule has a straddling
        // first period (premium.start strictly < T_exp); decoupling it from
        // the calibration anchor lets us reproduce Bloomberg's screen ATM
        // Fwd to within 0.05 bp without inducing a calibration shift in the
        // option NPV path.
        let display_denom_par =
            (display_annuity_at_value_dt * cds.notional.amount()).max(numerical::ZERO_TOLERANCE);
        let display_forward_par_spread = spot_protection_pv / display_denom_par;

        let coupon = decimal_to_f64(option.effective_underlying_cds_coupon())?;
        let strike = decimal_to_f64(option.strike)?;
        let scale = if option.underlying_is_index {
            option.index_factor.unwrap_or(1.0)
        } else {
            1.0
        };
        let realized_index_loss = option.realized_index_loss.unwrap_or(0.0);

        Ok(Self {
            lgd,
            t_expiry,
            sigma,
            df_to_expiry,
            survival_to_expiry,
            forward_par_spread,
            display_forward_par_spread,
            bootstrapped_l_at_expiry,
            times_from_expiry,
            accrual_factors,
            fwd_discount_factors,
            accrual_pcd_to_expiry,
            coupon,
            strike,
            option_type: option.option_type,
            scale,
            realized_index_loss,
            is_index: option.underlying_is_index,
        })
    }

    /// `ξ` per Eq. 2.1 / Eq. 2.4: `+1` payer, `−1` receiver.
    fn sign(&self) -> f64 {
        match self.option_type {
            OptionType::Call => 1.0,
            OptionType::Put => -1.0,
        }
    }

    /// Forward clean risky annuity *at expiry* under a flat hazard
    /// `λ = s / (1−R)`, evaluated on the synthetic CDS schedule:
    ///
    /// ```text
    /// L(s) = Σ_i α_i · exp(−λ · t_i_from_expiry) · fwd_df_i  −  α_pcd→te
    /// ```
    ///
    /// — the "credit triangle" simplification (DOCS 2055833 §2.5) lets us
    /// identify hazard with `s/(1−R)` directly so no per-node solve is
    /// needed inside the quadrature integrand. The PCD subtraction
    /// (`α_pcd→te`) corresponds to Bloomberg's "subtract the PV01 of the
    /// first cashflow" rule (HELP CDSO) reduced to its economically
    /// well-defined form: the option holder owes premium only from
    /// `T_e + 1` onward, so the pre-expiry portion of the period
    /// straddling expiry is netted out. For schedules where
    /// `premium.start = T_e` (no straddle), `accrual_pcd_to_expiry = 0`
    /// and this reduces to the raw post-expiry sum.
    fn flat_annuity(&self, s: f64) -> f64 {
        let lambda = s / self.lgd;
        let mut acc = -self.accrual_pcd_to_expiry;
        for ((alpha, t), fwd_df) in self
            .accrual_factors
            .iter()
            .zip(self.times_from_expiry.iter())
            .zip(self.fwd_discount_factors.iter())
        {
            let surv = (-lambda * t).exp();
            acc += alpha * surv * fwd_df;
        }
        acc
    }

    /// Per-unit-notional swap value at expiry under flat-spread `s`:
    /// `V_te(s)/N = (s − c) · L(s)`.
    fn swap_value_per_n(&self, s: f64) -> f64 {
        (s - self.coupon) * self.flat_annuity(s)
    }

    /// Eq. 2.4 deterministic strike adjustment, per unit notional.
    fn strike_adjustment_per_n(&self) -> f64 {
        self.sign() * (self.coupon - self.strike) * self.flat_annuity(self.strike)
    }

    /// Eq. 2.5 deterministic loss settlement, per unit current notional
    /// before the index-factor scale is applied.
    fn loss_settlement_per_n(&self) -> f64 {
        if !self.is_index || self.realized_index_loss <= 0.0 {
            return 0.0;
        }
        let scale = self.scale.max(numerical::ZERO_TOLERANCE);
        self.sign() * self.realized_index_loss / scale
    }

    /// Index CDS options are no-knockout; single-name CDS options knock out
    /// if default occurs before expiry.
    fn exercise_survival_multiplier(&self) -> f64 {
        if self.is_index {
            1.0
        } else {
            self.survival_to_expiry
        }
    }

    /// `F_0/N` — the no-knockout clean forward swap value, used as the
    /// calibration anchor for `m` (DOCS 2055833 Eq 2.3). For index options
    /// this includes the `(1−R)·(1−q_te)` FEP-equivalent contribution;
    /// single-name options (which knock out on default) drop that term.
    ///
    /// Note on L_te self-consistency: the integrand `V_te(s) = (s − c)·L(s)`
    /// uses the credit-triangle `L(s)` per DOCS 2055833 §2.5 (λ(s) =
    /// s/(1−R)), while F_0 uses the bootstrapped term-structure `L_te`.
    /// On cdx_ig_46 these differ by ~0.66% at par. Empirically, calibrating
    /// against the bootstrapped F_0 (current) gives NPV closer to the
    /// Bloomberg CDSO screen ($119,504 vs $118,782, +0.61%) than calibrating
    /// against the credit-triangle F_0 ($126,510, +6.5%) — suggesting the
    /// Bloomberg model intentionally combines a bootstrap-anchored F_0 with
    /// a credit-triangle integrand. Tightening below the 0.61% gap requires
    /// BBG-quant clarification on the exact L_te(s) interpolation.
    fn no_knockout_forward(&self) -> f64 {
        let h1 = if self.is_index {
            self.lgd * (1.0 - self.survival_to_expiry)
        } else {
            0.0
        };
        let h2 = (self.forward_par_spread - self.coupon) * self.bootstrapped_l_at_expiry;
        h1 + h2
    }
}

// =====================================================================
// Calibration of the lognormal mean `m` (DOCS 2055833 Eq. 2.3)
// =====================================================================

/// Solve the scalar nonlinear equation
///
/// ```text
/// E_0 [V_te(S_te(m))] = F_0
/// ```
///
/// where `S_te(m, ε) = m · exp(−½σ²t + σ√t·ε)`, `ε ∼ N(0, 1)`. Brent
/// root-finding in log-`m` space (positivity is enforced and the search is
/// well-conditioned across the realistic spread range).
fn calibrate_lognormal_mean(ctx: &ForwardCdsContext) -> Result<f64> {
    let target = ctx.no_knockout_forward();
    let t_expiry = ctx.t_expiry.max(0.0);
    let s0 = (-0.5 * ctx.sigma * ctx.sigma * t_expiry).exp();
    let sigma_sqrt_t = ctx.sigma * t_expiry.sqrt();

    let expected_v_te = |m: f64| -> f64 {
        normal_integral(Z_STEP, |z| {
            let s = m * s0 * (sigma_sqrt_t * z).exp();
            ctx.swap_value_per_n(s)
        })
    };

    let f = |log_m: f64| -> f64 { expected_v_te(log_m.exp()) - target };
    let solver = BrentSolver::new().tolerance(1e-12);
    let log_m = solver.solve_in_bracket(f, 1e-8_f64.ln(), 1.0_f64.ln())?;
    Ok(log_m.exp())
}

// =====================================================================
// Quadrature integrand (DOCS 2055833 Eq. 2.5)
// =====================================================================

/// `O / N = P(t_e) · E_0 [ (ξ V_te + H(K) + D)+ ]` per Eq. 2.5, evaluated
/// by trapezoidal rule on the standard normal density. The `scale` factor
/// folds in the index-factor adjustment for re-versioned indices.
fn price_with_calibrated_mean(ctx: &ForwardCdsContext, m: f64, t_expiry: f64) -> f64 {
    quadrature_payoff(
        ctx,
        m,
        ctx.strike_adjustment_per_n(),
        ctx.loss_settlement_per_n(),
        t_expiry,
    )
}

fn quadrature_payoff(ctx: &ForwardCdsContext, m: f64, h_k: f64, d_loss: f64, t_expiry: f64) -> f64 {
    let t_expiry = t_expiry.max(0.0);
    let s0 = (-0.5 * ctx.sigma * ctx.sigma * t_expiry).exp();
    let sigma_sqrt_t = ctx.sigma * t_expiry.sqrt();
    let sign = ctx.sign();

    let expected_payoff = normal_integral(Z_STEP, |z| {
        let s = m * s0 * (sigma_sqrt_t * z).exp();
        let v = ctx.swap_value_per_n(s); // V_te / N
        (sign * v + h_k + d_loss).max(0.0)
    });
    ctx.scale * ctx.exercise_survival_multiplier() * expected_payoff * ctx.df_to_expiry
}

// =====================================================================
// Internal utilities
// =====================================================================

fn decimal_to_f64(value: Decimal) -> Result<f64> {
    value.to_f64().ok_or_else(|| {
        finstack_core::Error::Validation(format!(
            "Bloomberg CDSO quadrature: cannot represent {value} as f64"
        ))
    })
}

fn normal_integral<F>(step: f64, mut value_at: F) -> f64
where
    F: FnMut(f64) -> f64,
{
    let n_steps = ((2.0 * Z_LIMIT) / step).round() as usize;
    let mut acc = 0.0;
    for i in 0..=n_steps {
        let z = -Z_LIMIT + (i as f64) * step;
        let weight = if i == 0 || i == n_steps { 0.5 } else { 1.0 };
        acc += weight * value_at(z) * (-0.5 * z * z).exp();
    }
    acc * INV_SQRT_2_PI * step
}

#[cfg(test)]
mod tests {
    use super::normal_integral;

    #[test]
    fn normal_quadrature_converges_when_step_is_halved() {
        let coarse = normal_integral(0.05, |z| (0.30 * z).exp().max(0.0));
        let fine = normal_integral(0.025, |z| (0.30 * z).exp().max(0.0));
        assert!(
            (coarse - fine).abs() < 1e-8,
            "normal quadrature should be stable under step halving: coarse={coarse}, fine={fine}",
        );
    }
}
