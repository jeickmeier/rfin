//! Bond quote engine for mapping between price, yields, and spreads.
//!
//! This module provides a small, opinionated API that takes **one**
//! quote input (price, yield, or spread) and produces a consistent
//! set of derived bond quotes using the existing pricing and metric
//! infrastructure.
//!
//! All spread-style quantities exposed here use **decimal units**:
//! `0.01` corresponds to **100 basis points**.

use crate::cashflow::traits::CashflowProvider;
use crate::instruments::bond::Bond;
use crate::instruments::common::traits::Instrument;
use crate::metrics::{standard_registry, MetricRegistry};
use crate::metrics::{MetricContext, MetricId};
use finstack_core::dates::Date;
use finstack_core::dates::DayCountCtx;
use finstack_core::market_data::MarketContext;
use finstack_core::money::Money;
use finstack_core::Result;
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
/// * `freq` - Payment frequency (e.g., `Frequency::semi_annual()`)
///
/// # Returns
///
/// Number of periods per year as `f64`.
///
/// # Errors
///
/// Returns `Err` when:
/// - Frequency is zero (invalid)
///
/// # Examples
///
/// ```rust
/// use finstack_valuations::instruments::bond::pricing::quote_engine::periods_per_year;
/// use finstack_core::dates::Frequency;
///
/// assert_eq!(periods_per_year(Frequency::semi_annual())?, 2.0);
/// assert_eq!(periods_per_year(Frequency::quarterly())?, 4.0);
/// assert_eq!(periods_per_year(Frequency::annual())?, 1.0);
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
///
/// # Note on Daily Frequency
///
/// For daily frequencies, this uses 365 as an approximation of annual periods.
/// This is appropriate for frequency calculations but should NOT be confused with
/// the Actual/365 day count convention used in accrual and discount factor calculations.
#[inline]
pub fn periods_per_year(freq: finstack_core::dates::Frequency) -> finstack_core::Result<f64> {
    match freq {
        finstack_core::dates::Frequency::Months(m) => {
            if m == 0 {
                return Err(finstack_core::error::InputError::Invalid.into());
            }
            Ok(12.0 / (m as f64))
        }
        finstack_core::dates::Frequency::Days(d) => {
            if d == 0 {
                return Err(finstack_core::error::InputError::Invalid.into());
            }
            // Use 365 as approximate annual basis for frequency calculations
            // Note: This is NOT a day count convention - actual day count is handled
            // via the DayCount enum (Actual/360, Actual/365, Actual/Actual, etc.)
            Ok(365.0 / (d as f64))
        }
        _ => Err(finstack_core::error::InputError::Invalid.into()),
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
/// use finstack_valuations::instruments::bond::pricing::quote_engine::fixed_leg_annuity;
/// use finstack_core::market_data::term_structures::discount_curve::DiscountCurve;
/// use finstack_core::dates::{DayCount, Date};
///
/// # let disc = DiscountCurve::builder("USD-OIS").base_date(Date::from_calendar_date(2024, time::Month::January, 1).unwrap()).knots([(0.0, 1.0)]).build().unwrap();
/// # let schedule = vec![Date::from_calendar_date(2024, time::Month::January, 1).unwrap(), Date::from_calendar_date(2025, time::Month::January, 1).unwrap()];
/// let annuity = fixed_leg_annuity(&disc, DayCount::Act365F, &schedule)?;
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
pub fn fixed_leg_annuity(
    disc: &finstack_core::market_data::term_structures::discount_curve::DiscountCurve,
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
        let p = disc.df_on_date_curve(d);
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
/// use finstack_valuations::instruments::bond::pricing::quote_engine::par_rate_and_annuity_from_discount;
/// use finstack_core::market_data::term_structures::discount_curve::DiscountCurve;
/// use finstack_core::dates::{DayCount, Date};
///
/// # let disc = DiscountCurve::builder("USD-OIS").base_date(Date::from_calendar_date(2024, time::Month::January, 1).unwrap()).knots([(0.0, 1.0)]).build().unwrap();
/// # let schedule = vec![Date::from_calendar_date(2024, time::Month::January, 1).unwrap(), Date::from_calendar_date(2025, time::Month::January, 1).unwrap()];
/// let (par_rate, annuity) = par_rate_and_annuity_from_discount(&disc, DayCount::Act365F, &schedule)?;
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
pub fn par_rate_and_annuity_from_discount(
    disc: &finstack_core::market_data::term_structures::discount_curve::DiscountCurve,
    dc: finstack_core::dates::DayCount,
    schedule: &[Date],
) -> finstack_core::Result<(f64, f64)> {
    if schedule.len() < 2 {
        return Ok((0.0, 0.0));
    }

    let ann = fixed_leg_annuity(disc, dc, schedule)?;
    if ann == 0.0 {
        return Ok((0.0, 0.0));
    }

    let p0 = disc.df_on_date_curve(schedule[0]);
    let pn = disc.df_on_date_curve(*schedule.last().expect("Schedule should not be empty"));
    let num = p0 - pn;
    Ok((num / ann, ann))
}

/// Quote input for the bond quote engine.
///
/// All spreads are expressed in **decimal** (`0.01 = 100bp`).
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
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum YieldCompounding {
    /// Simple variant.
    Simple,
    /// Annual variant.
    Annual,
    /// Periodic variant.
    Periodic(u32),
    /// Continuous variant.
    Continuous,
    /// Street variant.
    Street,
}

/// Discount factor from yield.
#[inline]
pub fn df_from_yield(
    ytm: f64,
    t: f64,
    comp: YieldCompounding,
    bond_freq: finstack_core::dates::Frequency,
) -> finstack_core::Result<f64> {
    if t <= 0.0 {
        return Ok(1.0);
    }
    Ok(match comp {
        YieldCompounding::Simple => 1.0 / (1.0 + ytm * t),
        YieldCompounding::Annual => (1.0 + ytm).powf(-t),
        YieldCompounding::Periodic(m) => {
            let m = m as f64;
            (1.0 + ytm / m).powf(-m * t)
        }
        YieldCompounding::Continuous => (-ytm * t).exp(),
        YieldCompounding::Street => {
            let m = periods_per_year(bond_freq)?.max(1.0);
            (1.0 + ytm / m).powf(-m * t)
        }
    })
}

/// Price from yield using explicit day count and frequency (no `Bond` borrow required).
#[inline]
pub fn price_from_ytm_compounded_params(
    day_count: finstack_core::dates::DayCount,
    freq: finstack_core::dates::Frequency,
    flows: &[(finstack_core::dates::Date, finstack_core::money::Money)],
    as_of: finstack_core::dates::Date,
    ytm: f64,
    comp: YieldCompounding,
) -> finstack_core::Result<f64> {
    let mut pv = 0.0;
    for &(date, amount) in flows {
        if date <= as_of {
            continue;
        }
        let t = day_count.year_fraction(as_of, date, DayCountCtx::default())?;
        if t > 0.0 {
            let df = df_from_yield(ytm, t, comp, freq)?;
            pv += amount.amount() * df;
        }
    }
    Ok(pv)
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

/// Solve yield-to-worst over all call/put/maturity candidates for a given flow set.
///
/// Returns the worst (minimum) yield and the corresponding truncated cashflow path.
pub(crate) fn solve_ytw_from_flows(
    bond: &Bond,
    flows: &[(Date, Money)],
    as_of: Date,
    dirty_price_target: Money,
) -> finstack_core::Result<(f64, Vec<(Date, Money)>)> {
    // Generate call/put candidates + maturity
    let mut candidates: Vec<(Date, Money)> = Vec::new();
    if let Some(cp) = &bond.call_put {
        for c in &cp.calls {
            if c.date >= as_of && c.date <= bond.maturity {
                candidates.push((c.date, bond.notional * (c.price_pct_of_par / 100.0)));
            }
        }
        for p in &cp.puts {
            if p.date >= as_of && p.date <= bond.maturity {
                candidates.push((p.date, bond.notional * (p.price_pct_of_par / 100.0)));
            }
        }
    }
    // At maturity, principal redemption is already present in the cashflow schedule,
    // so use a zero additional redemption here to avoid double-counting.
    candidates.push((bond.maturity, Money::new(0.0, bond.notional.currency())));

    let mut best_yield = f64::INFINITY;
    let mut best_flows: Vec<(Date, Money)> = Vec::new();

    for (exercise_date, redemption) in candidates {
        // Truncate flows to exercise and add redemption
        let mut ex_flows: Vec<(Date, Money)> = Vec::with_capacity(flows.len());
        for &(d, a) in flows {
            if d > as_of && d <= exercise_date {
                ex_flows.push((d, a));
            }
        }
        ex_flows.push((exercise_date, redemption));

        // Solve yield that matches target dirty price
        let coupon_rate = match &bond.cashflow_spec {
            crate::instruments::bond::CashflowSpec::Fixed(spec) => spec.rate,
            _ => 0.0,
        };
        let y = crate::instruments::bond::pricing::ytm_solver::solve_ytm(
            &ex_flows,
            as_of,
            dirty_price_target,
            crate::instruments::bond::pricing::ytm_solver::YtmPricingSpec {
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
    // Build holder-view flows and delegate to shared YTW helper
    let flows = bond.build_schedule(curves, as_of)?;
    let (best_yield, best_flows) = solve_ytw_from_flows(bond, &flows, as_of, dirty_price_target)?;

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
    let flows = bond.build_schedule(curves, as_of)?;
    let disc = curves.get_discount_ref(&bond.discount_curve_id)?;
    // Pre-compute as_of discount factor for correct theta using the curve's
    // own date mapping.
    let df_as_of = disc.df_on_date_curve(as_of);

    let mut pv = 0.0;
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

        // Discount from as_of using the curve's DF(date) mapping.
        let df_cf_abs = disc.df_on_date_curve(*d);
        let df = if df_as_of != 0.0 {
            df_cf_abs / df_as_of
        } else {
            1.0
        };
        let df_z = df * (-z * t_from_as_of).exp();
        pv += a.amount() * df_z;
    }
    Ok(pv)
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
    use crate::instruments::bond::pricing::tree_engine::BondValuator;
    use crate::instruments::common::models::{
        short_rate_keys, ShortRateTree, ShortRateTreeConfig, StateVariables, TreeModel,
    };
    // Time to maturity is measured from the valuation date (as_of) using the
    // discount curve's day-count to ensure consistency with tree calibration.
    let discount_curve = curves.get_discount_ref(&bond.discount_curve_id)?;
    let disc_dc = discount_curve.day_count();
    let time_to_maturity = disc_dc.year_fraction(as_of, bond.maturity, DayCountCtx::default())?;
    if time_to_maturity <= 0.0 {
        return Ok(0.0);
    }
    let mut short_rate_tree = ShortRateTree::new(ShortRateTreeConfig::default());
    short_rate_tree.calibrate(discount_curve, time_to_maturity)?;
    let valuator = BondValuator::new(bond.clone(), curves, as_of, time_to_maturity, 100)?;
    let mut vars = StateVariables::new();
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
        crate::instruments::bond::CashflowSpec::Floating(_)
    );
    if !is_floating {
        return Ok(bond.value(curves, as_of)?.amount());
    }
    let mut b = bond.clone();
    if let crate::instruments::bond::CashflowSpec::Floating(spec) = &mut b.cashflow_spec {
        spec.rate_spec.spread_bp += dm * 1e4;
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
/// use finstack_valuations::instruments::bond::Bond;
/// use finstack_valuations::instruments::bond::pricing::quote_engine::{compute_quotes, BondQuoteInput};
/// use finstack_core::market_data::MarketContext;
/// use finstack_core::dates::Date;
///
/// # let bond = Bond::example();
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

    // Accrued interest using generic cashflow accrual engine
    let schedule = bond_for_metrics.get_full_schedule(curves)?;
    let accrued_ccy = crate::cashflow::accrual::accrued_interest_amount(
        &schedule,
        as_of,
        &bond_for_metrics.accrual_config(),
    )?;

    let notional = bond_for_metrics.notional.amount();
    if notional == 0.0 {
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
            // Use standard holder-view flows and price_from_ytm helper.
            let flows =
                <Bond as CashflowProvider>::build_schedule(&bond_for_metrics, curves, as_of)?;
            let dirty_ccy = price_from_ytm(&bond_for_metrics, &flows, as_of, ytm)?;
            let clean_ccy = dirty_ccy - accrued_ccy;
            let clean_pct = clean_ccy / notional * 100.0;
            (clean_pct, clean_ccy, dirty_ccy)
        }
        BondQuoteInput::ZSpread(z) => {
            let dirty_ccy = price_from_z_spread(&bond_for_metrics, curves, as_of, z)?;
            let clean_ccy = dirty_ccy - accrued_ccy;
            let clean_pct = clean_ccy / notional * 100.0;
            (clean_pct, clean_ccy, dirty_ccy)
        }
        BondQuoteInput::DiscountMargin(dm) => {
            let dirty_ccy = price_from_dm(&bond_for_metrics, curves, as_of, dm)?;
            let clean_ccy = dirty_ccy - accrued_ccy;
            let clean_pct = clean_ccy / notional * 100.0;
            (clean_pct, clean_ccy, dirty_ccy)
        }
        BondQuoteInput::Oas(oas_decimal) => {
            let dirty_ccy = price_from_oas(&bond_for_metrics, curves, as_of, oas_decimal)?;
            let clean_ccy = dirty_ccy - accrued_ccy;
            let clean_pct = clean_ccy / notional * 100.0;
            (clean_pct, clean_ccy, dirty_ccy)
        }
        BondQuoteInput::AswMarket(asw_mkt) => {
            let dirty_ccy = price_from_asw_market(&bond_for_metrics, curves, as_of, asw_mkt)?;
            let clean_ccy = dirty_ccy - accrued_ccy;
            let clean_pct = clean_ccy / notional * 100.0;
            (clean_pct, clean_ccy, dirty_ccy)
        }
        BondQuoteInput::ISpread(i_spread) => {
            // I-spread = YTM - par_swap_rate → YTM = ISpread + par_swap_rate.
            let par_swap_rate = par_swap_rate_from_discount(bond, curves, as_of)?;
            let ytm = i_spread + par_swap_rate;
            let flows =
                <Bond as CashflowProvider>::build_schedule(&bond_for_metrics, curves, as_of)?;
            let dirty_ccy = price_from_ytm(&bond_for_metrics, &flows, as_of, ytm)?;
            let clean_ccy = dirty_ccy - accrued_ccy;
            let clean_pct = clean_ccy / notional * 100.0;
            (clean_pct, clean_ccy, dirty_ccy)
        }
    };

    // Stamp the canonical clean price quote into pricing_overrides so that all
    // existing metric calculators interpret this as the market price.
    bond_for_metrics.pricing_overrides.quoted_clean_price = Some(clean_price_pct);

    // 2) Build metric context and use the standard registry for the rest.
    let base_value = bond_for_metrics.value(curves, as_of)?;
    let registry: MetricRegistry = standard_registry();

    let instrument_arc: Arc<dyn Instrument> = Arc::new(bond_for_metrics.clone());
    let curves_arc = Arc::new(curves.clone());
    let mut ctx = MetricContext::new(instrument_arc, curves_arc, as_of, base_value);
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

    // We don't actually care about the HashMap return; we just want
    // the side-effect of populating `ctx.computed`.
    let _ = registry.compute(&metric_ids, &mut ctx)?;

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
    use finstack_core::dates::{DayCount, DayCountCtx, Frequency, ScheduleBuilder, StubKind};

    let disc = curves.get_discount_ref(&bond.discount_curve_id)?;

    // Mirror the schedule used in ISpreadCalculator (annual Act/Act, ShortFront stub).
    let dates: Vec<Date> = ScheduleBuilder::new(as_of, bond.maturity)
        .frequency(Frequency::annual())
        .stub_rule(StubKind::ShortFront)
        .build()?
        .into_iter()
        .collect();

    if dates.len() < 2 {
        return Ok(0.0);
    }

    let p0 = disc.df_on_date_curve(dates[0]);
    let pn = disc.df_on_date_curve(*dates.last().expect("Dates should not be empty"));
    let num = p0 - pn;
    let mut den = 0.0;
    for w in dates.windows(2) {
        let (a, b) = (w[0], w[1]);
        let alpha = DayCount::ActAct.year_fraction(a, b, DayCountCtx::default())?;
        let p = disc.df_on_date_curve(b);
        den += alpha * p;
    }
    if den == 0.0 {
        return Ok(0.0);
    }
    Ok(num / den)
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
    use crate::instruments::bond::CashflowSpec;
    use finstack_core::dates::calendar::calendar_by_id;
    use finstack_core::dates::ScheduleBuilder;

    // Only well-defined for fixed-rate, non-custom bonds in this helper.
    if bond.custom_cashflows.is_some() {
        return Err(finstack_core::error::InputError::Invalid.into());
    }
    let (coupon, freq, stub, bdc, calendar_id) = match &bond.cashflow_spec {
        CashflowSpec::Fixed(spec) => (
            spec.rate,
            spec.freq,
            spec.stub,
            spec.bdc,
            spec.calendar_id.as_deref(),
        ),
        _ => return Err(finstack_core::error::InputError::Invalid.into()),
    };

    let disc = curves.get_discount_ref(&bond.discount_curve_id)?;

    // Mirror the schedule and annuity definition used by AssetSwapMarketCalculator
    // (discount-ratio approximation on the fixed-leg schedule).
    let mut builder = ScheduleBuilder::new(as_of, bond.maturity)
        .frequency(freq)
        .stub_rule(stub);

    if let Some(id) = calendar_id {
        if let Some(cal) = calendar_by_id(id) {
            builder = builder.adjust_with(bdc, cal);
        }
    }

    let sched: Vec<Date> = builder.build()?.into_iter().collect();
    if sched.len() < 2 {
        return Ok(0.0);
    }

    let dc = bond.cashflow_spec.day_count();
    let (par_rate, ann) = par_rate_and_annuity_from_discount(disc, dc, &sched)?;
    if ann == 0.0 || bond.notional.amount() == 0.0 {
        return Ok(0.0);
    }

    let par_asw = coupon - par_rate;
    let price_pct = 1.0 + (asw_market - par_asw) * ann;
    Ok(price_pct * bond.notional.amount())
}
