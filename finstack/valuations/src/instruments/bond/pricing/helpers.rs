//! Bond pricing helpers (moved from bond/helpers.rs)

use super::super::types::Bond;
use crate::cashflow::traits::CashflowProvider;
use crate::instruments::common::traits::Instrument;
use finstack_core::dates::adjust;
use finstack_core::dates::calendar::calendar_by_id;
use finstack_core::dates::Date;
use finstack_core::dates::{BusinessDayConvention, DayCountCtx, StubKind};
use finstack_core::market_data::MarketContext;
use finstack_core::money::Money;

use time::Duration;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
/// Yield Compounding enumeration.
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

/// Convert payment frequency to approximate periods per year.
///
/// **Important:** This function is for **frequency conversion only**, NOT day count conventions.
///
/// # Purpose
/// This helper determines how many payment periods occur in a year based on the
/// payment frequency. For example, semi-annual payments occur 2 times per year,
/// monthly payments occur 12 times per year.
///
/// # Day Count Conventions
/// Actual day count calculations (Actual/360, Actual/365, Actual/Actual, 30/360, etc.)
/// are handled separately via the `DayCount` enum and `year_fraction()` methods in
/// finstack-core. Those methods properly account for:
/// - Leap years (Actual/Actual)
/// - Different day count bases (360 vs 365)
/// - Month length variations (30/360)
///
/// # Examples
/// - Monthly payments (6 months): `12 / 6 = 2` periods/year (semi-annual frequency)
/// - Daily payments (90 days): `365 / 90 ≈ 4.06` periods/year (approximate)
///
/// # Note on Daily Frequency
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

#[inline]
/// Df from yield.
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

/// price from ytm.
pub fn price_from_ytm(
    bond: &Bond,
    flows: &[(finstack_core::dates::Date, finstack_core::money::Money)],
    as_of: finstack_core::dates::Date,
    ytm: f64,
) -> finstack_core::Result<f64> {
    price_from_ytm_compounded(bond, flows, as_of, ytm, YieldCompounding::Street)
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

/// price from ytm compounded.
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
    candidates.push((
        bond.maturity,
        Money::new(0.0, bond.notional.currency()),
    ));

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
    let (best_yield, best_flows) =
        solve_ytw_from_flows(bond, &flows, as_of, dirty_price_target)?;

    // Re-price along the worst-yield path for a consistent price result
    let best_price =
        price_from_ytm_compounded(bond, &best_flows, as_of, best_yield, YieldCompounding::Street)?;

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
    _as_of: Date,
    oas_decimal: f64,
) -> finstack_core::Result<f64> {
    // Convert decimal spread (0.01 = 100bp) to basis points for the tree.
    let oas_bp = oas_decimal * 10_000.0;

    // Use the short-rate tree directly to price at a given OAS
    use crate::instruments::bond::pricing::tree_pricer::BondValuator;
    use crate::instruments::common::models::{
        short_rate_keys, ShortRateTree, ShortRateTreeConfig, StateVariables, TreeModel,
    };
    // Time to maturity is measured on the discount curve's own time basis so
    // that the short-rate tree is calibrated consistently with the curve.
    let discount_curve = curves.get_discount_ref(&bond.discount_curve_id)?;
    let disc_dc = discount_curve.day_count();
    let time_to_maturity = disc_dc.year_fraction(
        discount_curve.base_date(),
        bond.maturity,
        DayCountCtx::default(),
    )?;
    if time_to_maturity <= 0.0 {
        return Ok(0.0);
    }
    let mut short_rate_tree = ShortRateTree::new(ShortRateTreeConfig::default());
    short_rate_tree.calibrate(discount_curve, time_to_maturity)?;
    let valuator = BondValuator::new(bond.clone(), curves, time_to_maturity, 100)?;
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

/// Returns the default schedule parameters used across accrual/pricers to avoid duplication.
#[inline]
pub fn default_schedule_params() -> (StubKind, BusinessDayConvention, Option<&'static str>) {
    (StubKind::None, BusinessDayConvention::Following, None)
}

/// Calculate accrued interest using the specified accrual method.
///
/// Implements three market-standard accrual conventions:
/// - Linear: Simple interest interpolation (most common)
/// - Compounded: ICMA Rule 251 actuarial method
/// - Indexed: Inflation index ratio (for TIPS and similar)
fn calculate_accrual_by_method(
    accrual_method: &crate::instruments::bond::AccrualMethod,
    notional: f64,
    coupon_amount: f64,
    _coupon_rate: f64,
    total_period: f64,
    elapsed: f64,
    _curves: Option<&MarketContext>,
) -> finstack_core::Result<f64> {
    use crate::instruments::bond::AccrualMethod;
    
    if total_period <= 0.0 || elapsed < 0.0 {
        return Ok(0.0);
    }
    
    match accrual_method {
        AccrualMethod::Linear => {
            // Standard linear interpolation: Accrued = Coupon × (elapsed / period)
            Ok(coupon_amount * (elapsed / total_period))
        }
        AccrualMethod::Compounded { frequency: _ } => {
            // ICMA Rule 251 actuarial method
            // Accrued = Notional × [(1 + period_rate)^(elapsed/period) - 1]
            // where period_rate is the coupon payment divided by notional
            if notional <= 0.0 {
                return Ok(0.0);
            }
            
            // Calculate the period rate from the coupon amount
            let period_rate = coupon_amount / notional;
            
            if period_rate.abs() < 1e-12 {
                // Zero-coupon or near-zero rate: fall back to linear
                return Ok(coupon_amount * (elapsed / total_period));
            }
            
            let fraction = elapsed / total_period;
            let compound_factor = (1.0 + period_rate).powf(fraction);
            Ok(notional * (compound_factor - 1.0))
        }
        AccrualMethod::Indexed { index_curve_id } => {
            // Inflation-indexed bonds (TIPS-style) are modelled in a dedicated
            // inflation-linked bond instrument. Nominal `Bond` does not
            // implement full index-ratio accrual here to avoid silently
            // mis-pricing ILBs.
            let _ = index_curve_id; // suppress unused warning in this module
            Err(finstack_core::error::InputError::Invalid.into())
        }
    }
}

/// Locate the current coupon period for a given `as_of` date and apply ex-coupon rules.
///
/// Returns `Ok(None)` when:
/// - `as_of` falls outside all coupon windows, or
/// - `as_of` is inside the ex-coupon window for the next coupon (zero accrual).
fn find_coupon_window_with_ex_coupon(
    bond: &Bond,
    as_of: finstack_core::dates::Date,
    freq: finstack_core::dates::Frequency,
    stub: StubKind,
    bdc: BusinessDayConvention,
    calendar_id: Option<&str>,
) -> finstack_core::Result<
    Option<(
        finstack_core::dates::Date, // start
        finstack_core::dates::Date, // end
    )>,
> {
    let sched = crate::cashflow::builder::build_dates(
        bond.issue,
        bond.maturity,
        freq,
        stub,
        bdc,
        calendar_id,
    );

    for window in sched.dates.windows(2) {
        let start_date = window[0];
        let end_date = window[1];

        // If ex-coupon is set, treat dates within ex-coupon window as zero accrual
        if let Some(ex_days) = bond.ex_coupon_days {
            let ex_date = end_date - Duration::days(ex_days as i64);
            if as_of >= ex_date && as_of < end_date {
                return Ok(None);
            }
        }

        if start_date <= as_of && as_of < end_date {
            return Ok(Some((start_date, end_date)));
        }
    }

    Ok(None)
}

/// Compute accrued interest between the last and next coupon dates.
///
/// If custom cashflows exist, uses Fixed/Stub coupon flows for accrual; otherwise,
/// uses generated schedule based on bond fields and accrual method from bond spec.
///
/// Supports three accrual methods per ICMA standards:
/// - Linear (default): Simple interest interpolation
/// - Compounded: ICMA Rule 251 actuarial method
/// - Indexed: Index ratio interpolation for inflation-linked bonds
pub fn compute_accrued_interest(
    bond: &Bond,
    as_of: finstack_core::dates::Date,
) -> finstack_core::Result<f64> {
    use crate::cashflow::primitives::CFKind;
    // Prefer custom coupon flows when available
    if let Some(ref custom) = bond.custom_cashflows {
        let mut coupon_dates = Vec::new();
        for cf in &custom.flows {
            if cf.kind == CFKind::Fixed || cf.kind == CFKind::Stub {
                coupon_dates.push((cf.date, cf.amount));
            }
        }
        if coupon_dates.len() < 2 {
            return Ok(0.0);
        }
        for window in coupon_dates.windows(2) {
            let (start_date, _) = window[0];
            let (end_date, coupon_amount) = window[1];
            if start_date <= as_of && as_of < end_date {
                let dc = bond.cashflow_spec.day_count();
                let total_period =
                    dc.year_fraction(start_date, end_date, DayCountCtx::default())?;
                let elapsed = dc
                    .year_fraction(start_date, as_of, DayCountCtx::default())?
                    .max(0.0);
                    
                // Extract coupon rate from custom cashflow amount
                let coupon_rate = if bond.notional.amount() > 0.0 {
                    coupon_amount.amount() / bond.notional.amount()
                } else {
                    0.0
                };
                
                return calculate_accrual_by_method(
                    &bond.accrual_method,
                    bond.notional.amount(),
                    coupon_amount.amount(),
                    coupon_rate,
                    total_period,
                    elapsed,
                    None,
                );
            }
        }
        return Ok(0.0);
    }

    // Fallback to canonical schedule using bond fields
    // Extract schedule params from cashflow_spec
    let (freq, stub, bdc, calendar_id) = match &bond.cashflow_spec {
        crate::instruments::bond::CashflowSpec::Fixed(spec) => {
            (spec.freq, spec.stub, spec.bdc, spec.calendar_id.as_deref())
        }
        crate::instruments::bond::CashflowSpec::Floating(spec) => (
            spec.freq,
            spec.stub,
            spec.rate_spec.bdc,
            spec.rate_spec.calendar_id.as_deref(),
        ),
        crate::instruments::bond::CashflowSpec::Amortizing { base, .. } => match &**base {
            crate::instruments::bond::CashflowSpec::Fixed(spec) => {
                (spec.freq, spec.stub, spec.bdc, spec.calendar_id.as_deref())
            }
            crate::instruments::bond::CashflowSpec::Floating(spec) => (
                spec.freq,
                spec.stub,
                spec.rate_spec.bdc,
                spec.rate_spec.calendar_id.as_deref(),
            ),
            _ => return Err(finstack_core::error::InputError::Invalid.into()),
        },
    };
    if let Some((start_date, end_date)) =
        find_coupon_window_with_ex_coupon(bond, as_of, freq, stub, bdc, calendar_id)?
    {
        let dc = bond.cashflow_spec.day_count();
        let yf = dc.year_fraction(start_date, end_date, DayCountCtx::default())?;
        let coupon_rate = match &bond.cashflow_spec {
            crate::instruments::bond::CashflowSpec::Fixed(spec) => spec.rate,
            _ => 0.0,
        };
        let period_coupon = bond.notional.amount() * coupon_rate * yf;
        let elapsed = dc
            .year_fraction(start_date, as_of, DayCountCtx::default())?
            .max(0.0);

        return calculate_accrual_by_method(
            &bond.accrual_method,
            bond.notional.amount(),
            period_coupon,
            coupon_rate,
            yf,
            elapsed,
            None,
        );
    }

    Ok(0.0)
}

/// Context-aware accrued interest supporting FRNs by approximating the current
/// period coupon from the forward curve at the last reset date when needed.
pub fn compute_accrued_interest_with_context(
    bond: &Bond,
    curves: &MarketContext,
    as_of: finstack_core::dates::Date,
) -> finstack_core::Result<f64> {
    // If fixed or custom flows exist, fall back to standard helper and return
    let is_floating = matches!(
        &bond.cashflow_spec,
        crate::instruments::bond::CashflowSpec::Floating(_)
    );
    if !is_floating || bond.custom_cashflows.is_some() {
        return compute_accrued_interest(bond, as_of);
    }

    // FRN path: approximate accrual using forward rate fixed at last reset
    let (index_id, margin_bp, gearing, reset_lag_days, freq, stub, bdc, calendar_id, dc) =
        match &bond.cashflow_spec {
            crate::instruments::bond::CashflowSpec::Floating(spec) => (
                spec.rate_spec.index_id.as_str(),
                spec.rate_spec.spread_bp,
                spec.rate_spec.gearing,
                spec.rate_spec.reset_lag_days,
                spec.freq,
                spec.stub,
                spec.rate_spec.bdc,
                spec.rate_spec.calendar_id.as_deref(),
                spec.rate_spec.dc,
            ),
            _ => return compute_accrued_interest(bond, as_of),
        };
    let fwd = curves.get_forward_ref(index_id)?;

    // Build schedule with instrument conventions to locate current coupon window
    if let Some((start, end)) =
        find_coupon_window_with_ex_coupon(bond, as_of, freq, stub, bdc, calendar_id)?
    {
        // Determine reset date and forward time. If the reset date falls
        // *before* the forward curve base date (e.g., first period with
        // T‑2 reset lag and curve anchored at issue), clamp the time to
        // zero to avoid invalid date ranges while still using the base
        // curve level as the reset rate.
        let mut reset_date = start - Duration::days(reset_lag_days as i64);
        if let Some(id) = calendar_id {
            if let Some(cal) = calendar_by_id(id) {
                reset_date = adjust(reset_date, bdc, cal)?;
            }
        }
        let base_date = fwd.base_date();
        let t_reset = if reset_date <= base_date {
            0.0
        } else {
            fwd.day_count()
                .year_fraction(base_date, reset_date, DayCountCtx::default())?
        };
        let yf_total = dc.year_fraction(start, end, DayCountCtx::default())?;
        let yf_elapsed = dc
            .year_fraction(start, as_of, DayCountCtx::default())?
            .max(0.0);
        if yf_total <= 0.0 {
            return Ok(0.0);
        }
        let rate = gearing * fwd.rate(t_reset) + margin_bp * 1e-4;
        // Use current outstanding approximation as full notional for accrual
        let coupon_total = bond.notional.amount() * rate * yf_total;
        return Ok(coupon_total * (yf_elapsed / yf_total));
    }

    Ok(0.0)
}
