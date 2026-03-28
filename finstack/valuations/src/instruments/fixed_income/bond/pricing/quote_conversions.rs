//! Bond quote engine for mapping between price, yields, and spreads.
//!
//! This module provides a small, opinionated API that takes **one**
//! quote input (price, yield, or spread) and produces a consistent
//! set of derived bond quotes using the existing pricing and metric
//! infrastructure.
//!
//! All spread-style quantities exposed here use **decimal units**:
//! `0.01` corresponds to **100 basis points**.
use crate::constants::numerical::ZERO_TOLERANCE;
use crate::instruments::common_impl::traits::Instrument;
use crate::instruments::fixed_income::bond::pricing::settlement::QuoteDateContext;
use crate::instruments::fixed_income::bond::Bond;
use crate::metrics::{standard_registry, MetricRegistry};
use crate::metrics::{MetricContext, MetricId};
use finstack_core::dates::Date;
use finstack_core::dates::DayCountCtx;
use finstack_core::market_data::context::MarketContext;
use finstack_core::money::Money;
use finstack_core::Result;
use rust_decimal::prelude::ToPrimitive;
use std::sync::Arc;

/// Convert payment frequency to approximate periods per year.
///
/// **Important:** This function is for **frequency conversion only**, NOT day count conventions.
///
/// # Purpose
///
/// This helper determines how many payment periods occur in a year based on the
/// payment frequency. For example, semi-annual payments occur 2 times per year,
/// monthly payments occur 12 times per year.
///
/// # Day Count Conventions
///
/// Actual day count calculations (Actual/360, Actual/365, Actual/Actual, 30/360, etc.)
/// are handled separately via the `DayCount` enum and `year_fraction()` methods in
/// finstack-core. Those methods properly account for:
/// - Leap years (Actual/Actual)
/// - Different day count bases (360 vs 365)
/// - Month length variations (30/360)
///
/// # Arguments
///
/// * `freq` - Payment frequency (e.g., `Tenor::semi_annual()`)
///
/// # Returns
///
/// Number of periods per year as `f64`.
///
/// # Errors
///
/// Returns `Err` when:
/// - Tenor is zero (invalid)
///
/// # Examples
///
/// ```rust
/// use finstack_valuations::instruments::fixed_income::bond::pricing::quote_engine::periods_per_year;
/// use finstack_core::dates::Tenor;
///
/// assert_eq!(periods_per_year(Tenor::semi_annual())?, 2.0);
/// assert_eq!(periods_per_year(Tenor::quarterly())?, 4.0);
/// assert_eq!(periods_per_year(Tenor::annual())?, 1.0);
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
///
/// # Note on Daily Tenor
///
/// For daily frequencies, this uses 365 as an approximation of annual periods.
/// This is appropriate for frequency calculations but should NOT be confused with
/// the Actual/365 day count convention used in accrual and discount factor calculations.
#[inline]
pub fn periods_per_year(freq: finstack_core::dates::Tenor) -> finstack_core::Result<f64> {
    match freq.unit {
        finstack_core::dates::TenorUnit::Months => {
            if freq.count == 0 {
                return Err(finstack_core::InputError::Invalid.into());
            }
            Ok(12.0 / (freq.count as f64))
        }
        finstack_core::dates::TenorUnit::Days => {
            if freq.count == 0 {
                return Err(finstack_core::InputError::Invalid.into());
            }
            // Use 365 as approximate annual basis for frequency calculations
            // Note: This is NOT a day count convention - actual day count is handled
            // via the DayCount enum (Actual/360, Actual/365, Actual/Actual, etc.)
            Ok(365.0 / (freq.count as f64))
        }
        finstack_core::dates::TenorUnit::Years => {
            if freq.count == 0 {
                return Err(finstack_core::InputError::Invalid.into());
            }
            Ok(1.0 / (freq.count as f64))
        }
        finstack_core::dates::TenorUnit::Weeks => {
            if freq.count == 0 {
                return Err(finstack_core::InputError::Invalid.into());
            }
            Ok(52.0 / (freq.count as f64))
        }
    }
}

/// Fixed-leg annuity for a bond-style schedule using discount-curve discount factors.
///
/// This computes the standard swap-style annuity:
/// ```text
/// Annuity = Σ (α_i · P(as_of, T_i))
/// ```
/// where `α_i` is the year fraction between consecutive schedule dates under `dc`,
/// and `P(as_of, T_i)` is the discount factor from `as_of` to date `T_i`.
///
/// The `schedule` is expected to start at the valuation date (`as_of`) and
/// contain strictly increasing dates.
///
/// # Arguments
///
/// * `disc` - Discount curve for discount factor calculations
/// * `dc` - Day count convention for year fraction calculations
/// * `schedule` - Schedule of coupon payment dates (must start at `as_of`)
///
/// # Returns
///
/// The fixed-leg annuity value.
///
/// # Errors
///
/// Returns an error if any year_fraction calculation fails (e.g., invalid dates).
///
/// # Examples
///
/// ```rust,no_run
/// use finstack_valuations::instruments::fixed_income::bond::pricing::quote_engine::fixed_leg_annuity;
/// use finstack_core::market_data::term_structures::DiscountCurve;
/// use finstack_core::dates::{DayCount, Date};
///
/// # let disc = DiscountCurve::builder("USD-OIS").base_date(Date::from_calendar_date(2024, time::Month::January, 1).unwrap()).knots([(0.0, 1.0)]).build().unwrap();
/// # let schedule = vec![Date::from_calendar_date(2024, time::Month::January, 1).unwrap(), Date::from_calendar_date(2025, time::Month::January, 1).unwrap()];
/// let annuity = fixed_leg_annuity(&disc, DayCount::Act365F, &schedule)?;
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
pub fn fixed_leg_annuity(
    disc: &finstack_core::market_data::term_structures::DiscountCurve,
    dc: finstack_core::dates::DayCount,
    schedule: &[Date],
) -> finstack_core::Result<f64> {
    use finstack_core::dates::DayCountCtx;

    if schedule.len() < 2 {
        return Ok(0.0);
    }

    let mut ann = 0.0;
    let mut prev = schedule[0];
    for &d in &schedule[1..] {
        let alpha = dc.year_fraction(prev, d, DayCountCtx::default())?;
        let p = disc.df_on_date_curve(d)?;
        ann += alpha * p;
        prev = d;
    }
    Ok(ann)
}

/// Par swap rate from discount-curve discount ratios and a fixed-leg annuity.
///
/// Uses the standard discount-ratio formula:
/// ```text
/// par_rate = (P(as_of, T₀) - P(as_of, Tₙ)) / Annuity
/// ```
/// where the denominator is the fixed-leg annuity computed with `dc`.
///
/// Returns both the par rate and the annuity so callers can reuse the latter
/// in asset-swap formulas and related analytics.
///
/// # Arguments
///
/// * `disc` - Discount curve for discount factor calculations
/// * `dc` - Day count convention for year fraction calculations
/// * `schedule` - Schedule of coupon payment dates
///
/// # Returns
///
/// Tuple of `(par_rate, annuity)` where:
/// - `par_rate` is the par swap rate (decimal, e.g., 0.05 for 5%)
/// - `annuity` is the fixed-leg annuity value
///
/// # Errors
///
/// Returns an error if the annuity calculation fails (invalid dates/day-count).
///
/// # Examples
///
/// ```rust,no_run
/// use finstack_valuations::instruments::fixed_income::bond::pricing::quote_engine::par_rate_and_annuity_from_discount;
/// use finstack_core::market_data::term_structures::DiscountCurve;
/// use finstack_core::dates::{DayCount, Date};
///
/// # let disc = DiscountCurve::builder("USD-OIS").base_date(Date::from_calendar_date(2024, time::Month::January, 1).unwrap()).knots([(0.0, 1.0)]).build().unwrap();
/// # let schedule = vec![Date::from_calendar_date(2024, time::Month::January, 1).unwrap(), Date::from_calendar_date(2025, time::Month::January, 1).unwrap()];
/// let (par_rate, annuity) = par_rate_and_annuity_from_discount(&disc, DayCount::Act365F, &schedule)?;
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
pub fn par_rate_and_annuity_from_discount(
    disc: &finstack_core::market_data::term_structures::DiscountCurve,
    dc: finstack_core::dates::DayCount,
    schedule: &[Date],
) -> finstack_core::Result<(f64, f64)> {
    if schedule.len() < 2 {
        return Ok((0.0, 0.0));
    }

    let ann = fixed_leg_annuity(disc, dc, schedule)?;
    // Use epsilon check to avoid division by near-zero values that could amplify numerical noise
    if ann.abs() < 1e-12 {
        return Ok((0.0, 0.0));
    }

    let p0 = disc.df_on_date_curve(schedule[0])?;
    // `schedule.len() >= 2` by the guard above, so `schedule[0]` and `schedule[last]` are safe.
    let pn_date = schedule[schedule.len() - 1];
    let pn = disc.df_on_date_curve(pn_date)?;
    let num = p0 - pn;
    Ok((num / ann, ann))
}

/// Quote input for the bond quote engine.
///
/// All spreads are expressed in **decimal** (`0.01 = 100bp`).
#[derive(Debug, Clone, Copy)]
pub enum BondQuoteInput {
    /// Clean price quoted as percentage of par (e.g., 99.5 = 99.5% of par).
    CleanPricePct(f64),
    /// Dirty price in currency units.
    DirtyPriceCcy(f64),
    /// Yield to maturity (decimal).
    Ytm(f64),
    /// Z-spread over the discount curve (decimal).
    ZSpread(f64),
    /// Discount margin for FRNs (decimal).
    DiscountMargin(f64),
    /// Option-adjusted spread (decimal).
    Oas(f64),
    /// Asset swap market spread (decimal).
    AswMarket(f64),
    /// I-spread (decimal).
    ISpread(f64),
}

/// Full quote set produced by the quote engine.
///
/// - Prices are returned both in currency and as % of par.
/// - All spreads are decimal (`0.01 = 100bp`).
#[derive(Debug, Clone)]
pub struct BondQuoteSet {
    /// Clean price in currency.
    pub clean_price_ccy: f64,
    /// Clean price as percentage of par (quote convention).
    pub clean_price_pct: f64,
    /// Dirty price in currency.
    pub dirty_price_ccy: f64,
    /// Yield to maturity (decimal), if applicable.
    pub ytm: Option<f64>,
    /// Yield to worst (decimal), if applicable.
    pub ytw: Option<f64>,
    /// Z-spread over discount curve (decimal), if applicable.
    pub z_spread: Option<f64>,
    /// Discount margin for FRNs (decimal), if applicable.
    pub discount_margin: Option<f64>,
    /// Option-adjusted spread (decimal), if applicable.
    pub oas: Option<f64>,
    /// Asset swap par spread (decimal), if applicable.
    pub asw_par: Option<f64>,
    /// Asset swap market spread (decimal), if applicable.
    pub asw_market: Option<f64>,
    /// I-spread (decimal), if applicable.
    pub i_spread: Option<f64>,
}

// ============================================================================
// Price-from-Metric Functions
// ============================================================================

/// Yield Compounding enumeration.
///
/// Defines how yield-to-maturity is compounded when calculating present values.
/// Different markets and instrument types use different conventions.
///
/// # Market Standard Conventions
///
/// | Convention | Use Case | Formula |
/// |------------|----------|---------|
/// | `Street` | Most secondary market trading | `(1 + y/f)^(-f*t)` |
/// | `TreasuryActual` | US Treasury new issues with stubs | Simple interest for first period |
/// | `Simple` | Money market instruments | `1/(1 + y*t)` |
/// | `Continuous` | Theoretical/academic | `exp(-y*t)` |
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum YieldCompounding {
    /// Simple interest: `DF = 1 / (1 + y * t)`
    ///
    /// Used for money market instruments and short-dated securities.
    Simple,

    /// Annual compounding: `DF = (1 + y)^(-t)`
    Annual,

    /// Periodic compounding with explicit periods per year: `DF = (1 + y/m)^(-m*t)`
    Periodic(u32),

    /// Continuous compounding: `DF = exp(-y * t)`
    ///
    /// Used in theoretical models and some derivative pricing.
    Continuous,

    /// Street convention: periodic compounding aligned with bond's coupon frequency.
    ///
    /// This is the standard convention for secondary market bond trading.
    /// Formula: `DF = (1 + y/f)^(-f*t)` where `f` is coupon frequency.
    Street,

    /// ISDA/Treasury actual convention with simple interest for odd first period.
    ///
    /// Uses simple interest `1/(1 + y*t)` for the first (potentially irregular) period,
    /// then switches to periodic compounding for subsequent periods. This matches
    /// the official SEC/Treasury methodology for new issue pricing with stub periods.
    ///
    /// # When to Use
    ///
    /// - US Treasury new issues with short first coupons
    /// - Regulatory yield calculations requiring ISDA compliance
    /// - Benchmarking against official Bloomberg/Reuters Treasury yields
    ///
    /// # Typical Difference
    ///
    /// The difference vs `Street` convention is typically < 0.5 basis points for
    /// seasoned bonds, but can be 1-2 basis points for new issues with significant stubs.
    ///
    /// # Limitation
    ///
    /// Stub period detection is **time-based**, using `t < 1/frequency` as the criterion.
    /// This works correctly for standard bonds but may misclassify stubs on bonds with
    /// irregular first coupons that don't align with the standard frequency (e.g., a
    /// long-first stub spanning 8 months on a semi-annual bond).
    TreasuryActual,
}

/// Discount factor from yield.
///
/// Computes the discount factor for a given yield, time, and compounding convention.
///
/// # Arguments
///
/// * `ytm` - Yield to maturity as decimal (e.g., 0.05 for 5%)
/// * `t` - Time in years from valuation date to cashflow date
/// * `comp` - Compounding convention (see [`YieldCompounding`])
/// * `bond_freq` - Bond's coupon frequency (used for `Street` and `TreasuryActual`)
///
/// # Compounding Formulas
///
/// | Convention | Formula |
/// |------------|---------|
/// | Simple | `1 / (1 + y * t)` |
/// | Annual | `(1 + y)^(-t)` |
/// | Periodic(m) | `(1 + y/m)^(-m*t)` |
/// | Continuous | `exp(-y * t)` |
/// | Street | `(1 + y/f)^(-f*t)` where f = frequency |
/// | TreasuryActual | Simple for t < 1/f, then periodic |
///
/// # Errors
///
/// Returns `Err` if the bond frequency is invalid (zero periods).
///
/// # Negative Yields
///
/// Negative yields are supported for all compounding conventions. However:
/// - **Extreme negative yields** (< -50%) will log a warning as they often indicate
///   data or input errors.
/// - For periodic/annual compounding, yields more negative than `-m` (where `m` is
///   compounding frequency) would make `(1 + y/m)` negative, leading to `NaN` from
///   `powf`. Such cases return `Err`.
/// - Discount factors > 1.0 are mathematically valid for negative rates but unusual
///   in practice.
#[inline]
pub fn df_from_yield(
    ytm: f64,
    t: f64,
    comp: YieldCompounding,
    bond_freq: finstack_core::dates::Tenor,
) -> finstack_core::Result<f64> {
    if t <= 0.0 {
        return Ok(1.0);
    }

    // Warn on extreme negative yields which often indicate data errors
    if ytm < -0.5 {
        tracing::warn!(
            ytm = ytm,
            "Extreme negative yield detected (< -50%). This may indicate a data error."
        );
    }

    Ok(match comp {
        YieldCompounding::Simple => {
            let denom = 1.0 + ytm * t;
            // Check for non-positive denominator which would give invalid discount factor
            if denom <= 0.0 {
                return Err(finstack_core::Error::Validation(format!(
                    "Simple interest denominator (1 + y*t) = {} is non-positive for ytm={}, t={}",
                    denom, ytm, t
                )));
            }
            1.0 / denom
        }
        YieldCompounding::Annual => {
            let base = 1.0 + ytm;
            if base <= 0.0 {
                return Err(finstack_core::Error::Validation(format!(
                    "Annual compounding base (1 + y) = {} is non-positive for ytm={}",
                    base, ytm
                )));
            }
            base.powf(-t)
        }
        YieldCompounding::Periodic(m) => {
            let m = m as f64;
            let base = 1.0 + ytm / m;
            if base <= 0.0 {
                return Err(finstack_core::Error::Validation(format!(
                    "Periodic compounding base (1 + y/m) = {} is non-positive for ytm={}, m={}",
                    base, ytm, m
                )));
            }
            base.powf(-m * t)
        }
        YieldCompounding::Continuous => (-ytm * t).exp(),
        YieldCompounding::Street => {
            let m = periods_per_year(bond_freq)?.max(1.0);
            let base = 1.0 + ytm / m;
            if base <= 0.0 {
                return Err(finstack_core::Error::Validation(format!(
                    "Street compounding base (1 + y/m) = {} is non-positive for ytm={}, m={}",
                    base, ytm, m
                )));
            }
            base.powf(-m * t)
        }
        YieldCompounding::TreasuryActual => {
            // ISDA/Treasury actual convention:
            // - Use simple interest for the first (potentially irregular) period
            // - Use periodic compounding for subsequent full periods
            //
            // LIMITATION: Stub period detection is TIME-BASED, not SCHEDULE-AWARE.
            // We identify the first period as t < 1/frequency (i.e., less than
            // one full coupon period). This is a reasonable approximation that
            // captures the essence of the convention for standard bonds.
            //
            // For bonds with irregular first coupons that don't align with the
            // standard frequency (e.g., a long-first stub spanning 8 months on
            // a semi-annual bond), this heuristic may misclassify the stub.
            // For exact ISDA compliance with non-standard structures, consider
            // passing actual stub information from the cashflow schedule.
            let m = periods_per_year(bond_freq)?.max(1.0);
            let period_length = 1.0 / m;

            // Validate periodic compounding base for extreme negative yields
            let periodic_base = 1.0 + ytm / m;
            if periodic_base <= 0.0 {
                return Err(finstack_core::Error::Validation(format!(
                    "TreasuryActual periodic base (1 + y/m) = {} is non-positive for ytm={}, m={}",
                    periodic_base, ytm, m
                )));
            }

            if t <= period_length {
                // First (potentially stub) period: simple interest
                let denom = 1.0 + ytm * t;
                if denom <= 0.0 {
                    return Err(finstack_core::Error::Validation(format!(
                        "TreasuryActual simple interest denom (1 + y*t) = {} is non-positive for ytm={}, t={}",
                        denom, ytm, t
                    )));
                }
                1.0 / denom
            } else {
                // For subsequent periods, we need to compound:
                // - Simple interest for the first period portion
                // - Periodic compounding for the remaining full periods
                //
                // Total time t = stub_time + n_full_periods / m
                // where stub_time <= period_length
                //
                // DF = DF_stub * DF_periodic
                //    = 1/(1 + y*stub) * (1 + y/m)^(-n_full_periods)
                let n_full_periods = (t * m).floor();
                let stub_time = t - n_full_periods / m;

                if stub_time > 1e-10 {
                    // Has a stub period
                    let stub_denom = 1.0 + ytm * stub_time;
                    if stub_denom <= 0.0 {
                        return Err(finstack_core::Error::Validation(format!(
                            "TreasuryActual stub denom (1 + y*stub) = {} is non-positive for ytm={}, stub_time={}",
                            stub_denom, ytm, stub_time
                        )));
                    }
                    let df_stub = 1.0 / stub_denom;
                    let df_periodic = periodic_base.powf(-n_full_periods);
                    df_stub * df_periodic
                } else {
                    // No stub, pure periodic
                    periodic_base.powf(-m * t)
                }
            }
        }
    })
}

/// Price from yield using explicit day count and frequency (no `Bond` borrow required).
#[inline]
pub fn price_from_ytm_compounded_params(
    day_count: finstack_core::dates::DayCount,
    freq: finstack_core::dates::Tenor,
    flows: &[(finstack_core::dates::Date, finstack_core::money::Money)],
    as_of: finstack_core::dates::Date,
    ytm: f64,
    comp: YieldCompounding,
) -> finstack_core::Result<f64> {
    use finstack_core::math::summation::NeumaierAccumulator;

    let mut pv = NeumaierAccumulator::new();
    for &(date, amount) in flows {
        if date <= as_of {
            continue;
        }
        let t = day_count.year_fraction(as_of, date, DayCountCtx::default())?;
        if t > 0.0 {
            let df = df_from_yield(ytm, t, comp, freq)?;
            pv.add(amount.amount() * df);
        }
    }
    Ok(pv.total())
}

/// Price from ytm compounded.
pub fn price_from_ytm_compounded(
    bond: &Bond,
    flows: &[(finstack_core::dates::Date, finstack_core::money::Money)],
    as_of: finstack_core::dates::Date,
    ytm: f64,
    comp: YieldCompounding,
) -> finstack_core::Result<f64> {
    price_from_ytm_compounded_params(
        bond.cashflow_spec.day_count(),
        bond.cashflow_spec.frequency(),
        flows,
        as_of,
        ytm,
        comp,
    )
}

/// Price from ytm (using Street convention).
pub fn price_from_ytm(
    bond: &Bond,
    flows: &[(finstack_core::dates::Date, finstack_core::money::Money)],
    as_of: finstack_core::dates::Date,
    ytm: f64,
) -> finstack_core::Result<f64> {
    price_from_ytm_compounded(bond, flows, as_of, ytm, YieldCompounding::Street)
}

/// Compute outstanding principal at a given date from the cashflow schedule.
///
/// This is used by YTW and other yield calculations to determine the
/// redemption amount for amortizing callable/putable bonds.
fn outstanding_principal_at_date(
    schedule: &crate::cashflow::builder::CashFlowSchedule,
    target_date: Date,
) -> f64 {
    use crate::cashflow::primitives::CFKind;

    let initial = schedule.notional.initial.amount();
    let mut outstanding = initial;

    // Sum all amortization and principal payments up to (and including) target_date
    for cf in &schedule.flows {
        if cf.date > target_date {
            break;
        }
        if matches!(cf.kind, CFKind::Amortization | CFKind::Notional) && cf.amount.amount() > 0.0 {
            outstanding -= cf.amount.amount();
        }
    }

    outstanding.max(0.0)
}

/// Solve yield-to-worst over all call/put/maturity candidates for a given flow set.
///
/// Returns the worst (minimum) yield and the corresponding truncated cashflow path.
///
/// # Call/Put Redemption Convention
///
/// Call/put redemption prices are computed as `outstanding_principal × (price_pct_of_par / 100)`,
/// where `outstanding_principal` is the remaining principal at the exercise date after
/// any amortization. This correctly handles amortizing callable bonds and is consistent
/// with the tree-based OAS pricing.
///
/// # Arguments
///
/// * `bond` - The bond to calculate YTW for
/// * `flows` - Holder-view cashflows (coupons + principal)
/// * `as_of` - Valuation/quote date
/// * `dirty_price_target` - Target dirty price to match
/// * `schedule` - Optional full cashflow schedule for accurate outstanding principal
///   computation on amortizing bonds. When `None`, falls back to original notional.
pub(crate) fn solve_ytw_from_flows(
    bond: &Bond,
    flows: &[(Date, Money)],
    as_of: Date,
    dirty_price_target: Money,
    schedule: Option<&crate::cashflow::builder::CashFlowSchedule>,
) -> finstack_core::Result<(f64, Vec<(Date, Money)>)> {
    // Generate call/put candidates + maturity
    let mut candidates: Vec<(Date, Money)> = Vec::new();

    if let Some(cp) = &bond.call_put {
        for c in &cp.calls {
            if c.date >= as_of && c.date <= bond.maturity {
                candidates.push((
                    c.date,
                    Money::new(c.price_pct_of_par, bond.notional.currency()),
                ));
            }
        }
        for p in &cp.puts {
            if p.date >= as_of && p.date <= bond.maturity {
                candidates.push((
                    p.date,
                    Money::new(p.price_pct_of_par, bond.notional.currency()),
                ));
            }
        }
    }
    // At maturity, principal redemption is already present in the cashflow schedule,
    // so use a zero additional redemption here to avoid double-counting.
    candidates.push((bond.maturity, Money::new(0.0, bond.notional.currency())));

    let mut best_yield = f64::INFINITY;
    let mut best_flows: Vec<(Date, Money)> = Vec::new();

    for (exercise_date, pct_or_zero) in candidates {
        // Truncate flows to exercise and add redemption
        let mut ex_flows: Vec<(Date, Money)> = Vec::with_capacity(flows.len());
        for &(d, a) in flows {
            if d > as_of && d <= exercise_date {
                ex_flows.push((d, a));
            }
        }

        // Compute redemption amount:
        // - For maturity: pct is 0, so redemption is 0 (already in flows)
        // - For call/put: use outstanding principal at exercise date × (pct/100)
        let redemption = if pct_or_zero.amount() > 0.0 {
            // This is a call/put candidate, pct_or_zero holds the price_pct_of_par
            let pct = pct_or_zero.amount();
            // Use full schedule for accurate outstanding principal when available;
            // otherwise fall back to original notional (valid for bullet bonds).
            let outstanding = if let Some(sched) = schedule {
                outstanding_principal_at_date(sched, exercise_date)
            } else {
                bond.notional.amount()
            };
            Money::new(outstanding * (pct / 100.0), bond.notional.currency())
        } else {
            Money::new(0.0, bond.notional.currency())
        };
        ex_flows.push((exercise_date, redemption));

        // Solve yield that matches target dirty price
        let coupon_rate = match &bond.cashflow_spec {
            crate::instruments::fixed_income::bond::CashflowSpec::Fixed(spec) => {
                spec.rate.to_f64().unwrap_or(0.0)
            }
            _ => 0.0,
        };
        let y = crate::instruments::fixed_income::bond::pricing::ytm_solver::solve_ytm(
            &ex_flows,
            as_of,
            dirty_price_target,
            crate::instruments::fixed_income::bond::pricing::ytm_solver::YtmPricingSpec {
                day_count: bond.cashflow_spec.day_count(),
                notional: bond.notional,
                coupon_rate,
                compounding: YieldCompounding::Street,
                frequency: bond.cashflow_spec.frequency(),
            },
        )?;

        if y < best_yield {
            best_yield = y;
            best_flows = ex_flows;
        }
    }

    Ok((best_yield, best_flows))
}

/// Price from Yield-To-Worst by scanning call/put candidates and selecting the lowest yield path.
pub fn price_from_ytw(
    bond: &Bond,
    curves: &MarketContext,
    as_of: Date,
    dirty_price_target: Money,
) -> finstack_core::Result<f64> {
    // Build signed canonical schedule flows and full schedule for accurate amortizing bond handling
    let flows = bond.pricing_dated_cashflows(curves, as_of)?;
    let schedule = bond.full_cashflow_schedule(curves)?;
    let (best_yield, best_flows) =
        solve_ytw_from_flows(bond, &flows, as_of, dirty_price_target, Some(&schedule))?;

    // Re-price along the worst-yield path for a consistent price result
    let best_price = price_from_ytm_compounded(
        bond,
        &best_flows,
        as_of,
        best_yield,
        YieldCompounding::Street,
    )?;

    Ok(best_price)
}

/// Price from Z-spread applied exponentially to base discount curve
pub fn price_from_z_spread(
    bond: &Bond,
    curves: &MarketContext,
    as_of: Date,
    z: f64,
) -> finstack_core::Result<f64> {
    use finstack_core::math::summation::NeumaierAccumulator;

    let flows = bond.pricing_dated_cashflows(curves, as_of)?;
    let disc = curves.get_discount(&bond.discount_curve_id)?;

    let mut pv = NeumaierAccumulator::new();
    for (d, a) in &flows {
        if *d <= as_of {
            continue;
        }
        // Time from as_of used for the exponential z-spread term is measured
        // on the same basis as the discount curve to keep the spread
        // definition aligned with the curve's own time axis.
        let t_from_as_of = disc
            .day_count()
            .year_fraction(as_of, *d, DayCountCtx::default())?;

        let df = disc.df_between_dates(as_of, *d)?;
        let df_z = df * (-z * t_from_as_of).exp();
        pv.add(a.amount() * df_z);
    }
    Ok(pv.total())
}

/// Price from Option-Adjusted Spread using the short-rate tree pricer.
///
/// The public API takes **decimal spread units** (`oas_decimal`), where
/// `0.01` corresponds to **100 basis points**. Internally, the tree
/// pricer continues to work in basis points for compatibility, so we
/// convert:
///
/// - `oas_bp = oas_decimal * 10_000.0`
///
/// This keeps all bond spread-style metrics on a consistent decimal
/// convention at the API surface while preserving existing internal
/// tree semantics.
pub fn price_from_oas(
    bond: &Bond,
    curves: &MarketContext,
    as_of: Date,
    oas_decimal: f64,
) -> finstack_core::Result<f64> {
    // Convert decimal spread (0.01 = 100bp) to basis points for the tree.
    let oas_bp = oas_decimal * 10_000.0;

    // Use the short-rate tree directly to price at a given OAS
    use crate::instruments::common_impl::models::{
        short_rate_keys, ShortRateTree, ShortRateTreeConfig, StateVariables, TreeModel,
    };
    use crate::instruments::fixed_income::bond::pricing::tree_engine::{
        bond_tree_config, BondValuator,
    };
    // Time to maturity is measured from the valuation date (as_of) using the
    // discount curve's day-count to ensure consistency with tree calibration.
    let discount_curve = curves.get_discount(&bond.discount_curve_id)?;
    let disc_dc = discount_curve.day_count();
    let time_to_maturity = disc_dc.year_fraction(as_of, bond.maturity, DayCountCtx::default())?;
    if time_to_maturity <= 0.0 {
        return Ok(0.0);
    }
    // Use bond_tree_config to source tree parameters from pricing_overrides,
    // ensuring round-trip consistency with calculate_oas() and value_with_tree().
    let config = bond_tree_config(bond);
    let tree_steps = config.tree_steps;
    let mut short_rate_tree = ShortRateTree::new(ShortRateTreeConfig {
        steps: config.tree_steps,
        volatility: config.volatility,
        mean_reversion: config.mean_reversion,
        ..Default::default()
    });
    short_rate_tree.calibrate(discount_curve.as_ref(), time_to_maturity)?;
    let valuator = BondValuator::new(bond.clone(), curves, as_of, time_to_maturity, tree_steps)?;

    // Get initial short rate from the calibrated tree for the state variables.
    // The tree framework expects an initial rate to be present.
    let initial_rate = short_rate_tree.rate_at_node(0, 0)?;
    let mut vars = StateVariables::default();
    vars.insert(short_rate_keys::SHORT_RATE, initial_rate);
    vars.insert(short_rate_keys::OAS, oas_bp);

    let price = short_rate_tree.price(vars, time_to_maturity, curves, &valuator)?;
    Ok(price)
}

/// Price from Discount Margin for FRNs by adding DM (decimal) to float margin and delegating to pricer
pub fn price_from_dm(
    bond: &Bond,
    curves: &MarketContext,
    as_of: Date,
    dm: f64,
) -> finstack_core::Result<f64> {
    // Check if it's a floating rate bond
    let is_floating = matches!(
        &bond.cashflow_spec,
        crate::instruments::fixed_income::bond::CashflowSpec::Floating(_)
    );
    if !is_floating {
        return Ok(bond.value(curves, as_of)?.amount());
    }
    let mut b = bond.clone();
    if let crate::instruments::fixed_income::bond::CashflowSpec::Floating(spec) =
        &mut b.cashflow_spec
    {
        // Convert dm (in decimal) to basis points and add to spread_bp (Decimal)
        let dm_bp = crate::utils::decimal::f64_to_decimal(dm * 1e4, "dm")?;
        spec.rate_spec.spread_bp += dm_bp;
    }
    Ok(b.value(curves, as_of)?.amount())
}

// ============================================================================
// Main Quote Engine
// ============================================================================

/// Convert between price, yield, and spread metrics for a bond.
///
/// The engine:
/// - Normalizes the chosen `quote_input` into a **canonical dirty price in currency**.
/// - Derives the corresponding clean price (% of par) and stamps it into
///   `pricing_overrides.quoted_clean_price` on an internal bond clone.
/// - Uses the standard metrics registry to compute the remaining metrics.
///
/// # Arguments
///
/// * `bond` - The bond to compute quotes for
/// * `curves` - Market context with discount and forward curves
/// * `as_of` - Valuation date
/// * `quote_input` - One quote input (price, yield, or spread) to normalize from
///
/// # Returns
///
/// A `BondQuoteSet` containing all computed price, yield, and spread metrics.
///
/// # Errors
///
/// Returns `Err` when:
/// - Market curves are missing
/// - Cashflow schedule building fails
/// - Metric calculations fail
///
/// # Examples
///
/// ```rust,no_run
/// use finstack_valuations::instruments::fixed_income::bond::Bond;
/// use finstack_valuations::instruments::fixed_income::bond::pricing::quote_engine::{compute_quotes, BondQuoteInput};
/// use finstack_core::market_data::context::MarketContext;
/// use finstack_core::dates::Date;
///
/// # let bond = Bond::example().unwrap();
/// # let curves = MarketContext::new();
/// # let as_of = Date::from_calendar_date(2024, time::Month::January, 15).unwrap();
/// let quotes = compute_quotes(&bond, &curves, as_of, BondQuoteInput::CleanPricePct(98.5))?;
/// // quotes contains YTM, Z-spread, OAS, etc.
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
pub fn compute_quotes(
    bond: &Bond,
    curves: &MarketContext,
    as_of: Date,
    quote_input: BondQuoteInput,
) -> Result<BondQuoteSet> {
    // Work on a local clone so we never mutate the caller's bond instance.
    let mut bond_for_metrics = bond.clone();

    // Quote normalization (clean/dirty conversion) must use accrued at quote/settlement date.
    let quote_ctx = QuoteDateContext::new(&bond_for_metrics, curves, as_of)?;
    let accrued_ccy = quote_ctx.accrued_at_quote_date;

    let notional = bond_for_metrics.notional.amount();
    if notional.abs() < ZERO_TOLERANCE {
        return Ok(BondQuoteSet {
            clean_price_ccy: 0.0,
            clean_price_pct: 0.0,
            dirty_price_ccy: 0.0,
            ytm: None,
            ytw: None,
            z_spread: None,
            discount_margin: None,
            oas: None,
            asw_par: None,
            asw_market: None,
            i_spread: None,
        });
    }

    // 1) Normalize the input into canonical dirty + clean prices.
    let (clean_price_pct, clean_price_ccy, dirty_price_ccy) = match quote_input {
        BondQuoteInput::CleanPricePct(clean_pct) => {
            let clean_ccy = clean_pct * notional / 100.0;
            let dirty_ccy = clean_ccy + accrued_ccy;
            (clean_pct, clean_ccy, dirty_ccy)
        }
        BondQuoteInput::DirtyPriceCcy(dirty_ccy) => {
            let clean_ccy = dirty_ccy - accrued_ccy;
            let clean_pct = clean_ccy / notional * 100.0;
            (clean_pct, clean_ccy, dirty_ccy)
        }
        BondQuoteInput::Ytm(ytm) => {
            // Use standard signed canonical schedule flows and price_from_ytm helper.
            let flows = bond_for_metrics.pricing_dated_cashflows(curves, as_of)?;
            let dirty_ccy = price_from_ytm(&bond_for_metrics, &flows, quote_ctx.quote_date, ytm)?;
            let clean_ccy = dirty_ccy - accrued_ccy;
            let clean_pct = clean_ccy / notional * 100.0;
            (clean_pct, clean_ccy, dirty_ccy)
        }
        BondQuoteInput::ZSpread(z) => {
            let dirty_ccy =
                price_from_z_spread(&bond_for_metrics, curves, quote_ctx.quote_date, z)?;
            let clean_ccy = dirty_ccy - accrued_ccy;
            let clean_pct = clean_ccy / notional * 100.0;
            (clean_pct, clean_ccy, dirty_ccy)
        }
        BondQuoteInput::DiscountMargin(dm) => {
            let dirty_ccy = price_from_dm(&bond_for_metrics, curves, quote_ctx.quote_date, dm)?;
            let clean_ccy = dirty_ccy - accrued_ccy;
            let clean_pct = clean_ccy / notional * 100.0;
            (clean_pct, clean_ccy, dirty_ccy)
        }
        BondQuoteInput::Oas(oas_decimal) => {
            let dirty_ccy =
                price_from_oas(&bond_for_metrics, curves, quote_ctx.quote_date, oas_decimal)?;
            let clean_ccy = dirty_ccy - accrued_ccy;
            let clean_pct = clean_ccy / notional * 100.0;
            (clean_pct, clean_ccy, dirty_ccy)
        }
        BondQuoteInput::AswMarket(asw_mkt) => {
            let dirty_ccy =
                price_from_asw_market(&bond_for_metrics, curves, quote_ctx.quote_date, asw_mkt)?;
            let clean_ccy = dirty_ccy - accrued_ccy;
            let clean_pct = clean_ccy / notional * 100.0;
            (clean_pct, clean_ccy, dirty_ccy)
        }
        BondQuoteInput::ISpread(i_spread) => {
            // I-spread = YTM - par_swap_rate → YTM = ISpread + par_swap_rate.
            let par_swap_rate = par_swap_rate_from_discount(bond, curves, as_of)?;
            let ytm = i_spread + par_swap_rate;
            let flows = bond_for_metrics.pricing_dated_cashflows(curves, as_of)?;
            let dirty_ccy = price_from_ytm(&bond_for_metrics, &flows, quote_ctx.quote_date, ytm)?;
            let clean_ccy = dirty_ccy - accrued_ccy;
            let clean_pct = clean_ccy / notional * 100.0;
            (clean_pct, clean_ccy, dirty_ccy)
        }
    };

    // Stamp the canonical clean price quote into pricing_overrides so that all
    // existing metric calculators interpret this as the market price.
    bond_for_metrics
        .pricing_overrides
        .market_quotes
        .quoted_clean_price = Some(clean_price_pct);

    // 2) Build metric context and use the standard registry for the rest.
    let base_value = bond_for_metrics.value(curves, as_of)?;
    let registry: MetricRegistry = standard_registry().clone();

    let instrument_arc: Arc<dyn Instrument> = Arc::new(bond_for_metrics.clone());
    let curves_arc = Arc::new(curves.clone());
    let mut ctx = MetricContext::new(
        instrument_arc,
        curves_arc,
        as_of,
        base_value,
        MetricContext::default_config(),
    );
    ctx.notional = Some(bond_for_metrics.notional);

    // Pre-populate accrued since we've already computed it.
    ctx.computed.insert(MetricId::Accrued, accrued_ccy);

    // Request the core price/yield/spread metrics.
    let metric_ids = [
        MetricId::Ytm,
        MetricId::Ytw,
        MetricId::ZSpread,
        MetricId::DiscountMargin,
        MetricId::Oas,
        MetricId::ASWPar,
        MetricId::ASWMarket,
        MetricId::ISpread,
    ];

    // Some quote metrics are not applicable to all bond types (e.g. FRN vs fixed),
    // and we want `compute_quotes` to return whatever is available rather than
    // failing the entire quote set.
    for metric_id in metric_ids.iter() {
        if let Err(err) = registry.compute(std::slice::from_ref(metric_id), &mut ctx) {
            tracing::debug!(
                metric_id = metric_id.as_str(),
                error = %err,
                "Bond quote engine metric computation failed; leaving unset"
            );
        }
    }

    // Read back the metrics we care about.
    let ytm = ctx.computed.get(&MetricId::Ytm).copied();
    let ytw = ctx.computed.get(&MetricId::Ytw).copied();
    let z_spread = ctx.computed.get(&MetricId::ZSpread).copied();
    let discount_margin = ctx.computed.get(&MetricId::DiscountMargin).copied();
    let oas = ctx.computed.get(&MetricId::Oas).copied();
    let asw_par = ctx.computed.get(&MetricId::ASWPar).copied();
    let asw_market = ctx.computed.get(&MetricId::ASWMarket).copied();
    let i_spread = ctx.computed.get(&MetricId::ISpread).copied();

    Ok(BondQuoteSet {
        clean_price_ccy,
        clean_price_pct,
        dirty_price_ccy,
        ytm,
        ytw,
        z_spread,
        discount_margin,
        oas,
        asw_par,
        asw_market,
        i_spread,
    })
}

/// Compute the par swap fixed rate used in the I-Spread definition
/// (`ISpread = YTM - par_swap_rate`) using the same convention as the
/// `ISpreadCalculator` (annual Act/Act proxy fixed leg by default).
fn par_swap_rate_from_discount(bond: &Bond, curves: &MarketContext, as_of: Date) -> Result<f64> {
    use finstack_core::dates::{ScheduleBuilder, StubKind};

    let disc = curves.get_discount(&bond.discount_curve_id)?;
    let ispread_cfg =
        crate::instruments::fixed_income::bond::metrics::price_yield_spread::i_spread::ISpreadConfig::default();

    // Mirror the fallback logic in `ISpreadCalculator`:
    // when using the default (annual Act/Act) proxy-leg config, use the bond's
    // fixed-coupon conventions for the proxy fixed leg.
    let mut fixed_leg_day_count = ispread_cfg.fixed_leg_day_count;
    let mut fixed_leg_frequency = ispread_cfg.fixed_leg_frequency;
    if matches!(
        ispread_cfg.fixed_leg_day_count,
        finstack_core::dates::DayCount::ActAct
    ) && ispread_cfg.fixed_leg_frequency == finstack_core::dates::Tenor::annual()
    {
        if let crate::instruments::fixed_income::bond::CashflowSpec::Fixed(spec) =
            &bond.cashflow_spec
        {
            fixed_leg_day_count = spec.dc;
            fixed_leg_frequency = spec.freq;
        }
    }

    // Mirror the schedule and fixed-leg conventions used in ISpreadCalculator defaults.
    let dates: Vec<Date> = ScheduleBuilder::new(as_of, bond.maturity)?
        .frequency(fixed_leg_frequency)
        .stub_rule(StubKind::ShortFront)
        .build()?
        .into_iter()
        .collect();

    if dates.len() < 2 {
        return Err(finstack_core::Error::Validation(
            "I-spread proxy par-swap calculation requires at least two schedule dates".to_string(),
        ));
    }

    let (par_rate, annuity) =
        par_rate_and_annuity_from_discount(disc.as_ref(), fixed_leg_day_count, &dates)?;
    if annuity.abs() < 1e-12 {
        return Err(finstack_core::Error::Validation(
            "I-spread proxy par-swap calculation is undefined for near-zero annuity".to_string(),
        ));
    }
    Ok(par_rate)
}

/// Price from market asset swap spread (decimal) using the same
/// approximation as `AssetSwapMarketCalculator` for non-custom,
/// fixed-rate bonds:
///
/// `ASW_mkt = (coupon - par_rate) + (price_pct - 1.0) / annuity`
///
/// where `price_pct = dirty / notional`. Inverting:
///
/// `price_pct = 1.0 + (ASW_mkt - (coupon - par_rate)) * annuity`.
fn price_from_asw_market(
    bond: &Bond,
    curves: &MarketContext,
    as_of: Date,
    asw_market: f64,
) -> Result<f64> {
    use crate::instruments::fixed_income::bond::CashflowSpec;
    use finstack_core::dates::calendar::calendar_by_id;
    use finstack_core::dates::ScheduleBuilder;

    // Only well-defined for fixed-rate, non-custom bonds in this helper.
    if bond.custom_cashflows.is_some() {
        return Err(finstack_core::InputError::Invalid.into());
    }
    let (coupon, freq, stub, bdc, calendar_id) = match &bond.cashflow_spec {
        CashflowSpec::Fixed(spec) => (
            spec.rate.to_f64().unwrap_or(0.0),
            spec.freq,
            spec.stub,
            spec.bdc,
            Some(spec.calendar_id.as_str()),
        ),
        _ => return Err(finstack_core::InputError::Invalid.into()),
    };

    let disc = curves.get_discount(&bond.discount_curve_id)?;

    // Mirror the schedule and annuity definition used by AssetSwapMarketCalculator
    // (discount-ratio approximation on the fixed-leg schedule).
    if as_of >= bond.maturity {
        return Err(finstack_core::Error::Validation(
            "ASW market price inversion requires at least two fixed-leg schedule dates".to_string(),
        ));
    }
    let mut builder = ScheduleBuilder::new(as_of, bond.maturity)?
        .frequency(freq)
        .stub_rule(stub);

    if let Some(id) = calendar_id {
        if let Some(cal) = calendar_by_id(id) {
            builder = builder.adjust_with(bdc, cal);
        }
    }

    let sched: Vec<Date> = builder.build()?.into_iter().collect();
    if sched.len() < 2 {
        return Err(finstack_core::Error::Validation(
            "ASW market price inversion requires at least two fixed-leg schedule dates".to_string(),
        ));
    }

    let dc = bond.cashflow_spec.day_count();
    let (par_rate, ann) = par_rate_and_annuity_from_discount(disc.as_ref(), dc, &sched)?;
    if bond.notional.amount().abs() < 1e-12 {
        return Err(finstack_core::Error::Validation(
            "ASW market price inversion is undefined for near-zero notional".to_string(),
        ));
    }
    // Use epsilon check to avoid unstable inversion when annuity is degenerate.
    if ann.abs() < 1e-12 {
        return Err(finstack_core::Error::Validation(
            "ASW market price inversion is undefined for near-zero fixed-leg annuity".to_string(),
        ));
    }

    let par_asw = coupon - par_rate;
    let price_pct = 1.0 + (asw_market - par_asw) * ann;
    Ok(price_pct * bond.notional.amount())
}

#[cfg(test)]
#[allow(clippy::expect_used, clippy::panic)]
mod tests {
    use super::*;
    use crate::instruments::fixed_income::bond::Bond;
    use finstack_core::currency::Currency;
    use finstack_core::market_data::term_structures::DiscountCurve;
    use finstack_core::money::Money;
    use time::macros::date;

    #[test]
    fn compute_quotes_returns_zeroes_for_effectively_zero_notional() {
        let as_of = date!(2025 - 01 - 01);
        let bond = Bond::fixed(
            "QE-NEAR-ZERO-NOTIONAL",
            Money::new(1e-12, Currency::USD),
            0.05,
            as_of,
            date!(2030 - 01 - 01),
            "USD-OIS",
        )
        .expect("bond");
        let curve = DiscountCurve::builder("USD-OIS")
            .base_date(as_of)
            .knots([(0.0, 1.0), (5.0, 0.8)])
            .build()
            .expect("curve");

        let quotes = compute_quotes(
            &bond,
            &MarketContext::new().insert(curve),
            as_of,
            BondQuoteInput::CleanPricePct(99.0),
        )
        .expect("quote conversion");

        assert_eq!(quotes.clean_price_ccy, 0.0);
        assert_eq!(quotes.clean_price_pct, 0.0);
        assert_eq!(quotes.dirty_price_ccy, 0.0);
        assert!(quotes.ytm.is_none());
    }
}
