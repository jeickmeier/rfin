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
pub enum YieldCompounding {
    Simple,
    Annual,
    Periodic(u32),
    Continuous,
    Street,
}

#[inline]
pub fn periods_per_year(
    freq: finstack_core::dates::Frequency,
) -> finstack_core::Result<f64> {
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
            Ok(365.0 / (d as f64))
        }
        _ => Err(finstack_core::error::InputError::Invalid.into()),
    }
}

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
        let t = day_count
            .year_fraction(as_of, date, DayCountCtx::default())
            .unwrap_or(0.0);
        if t > 0.0 {
            let df = df_from_yield(ytm, t, comp, freq)?;
            pv += amount.amount() * df;
        }
    }
    Ok(pv)
}

pub fn price_from_ytm_compounded(
    bond: &Bond,
    flows: &[(finstack_core::dates::Date, finstack_core::money::Money)],
    as_of: finstack_core::dates::Date,
    ytm: f64,
    comp: YieldCompounding,
) -> finstack_core::Result<f64> {
    price_from_ytm_compounded_params(bond.dc, bond.freq, flows, as_of, ytm, comp)
}

/// Price from Yield-To-Worst by scanning call/put candidates and selecting the lowest yield path
pub fn price_from_ytw(
    bond: &Bond,
    curves: &MarketContext,
    as_of: Date,
    dirty_price_target: Money,
) -> finstack_core::Result<f64> {
    // Build or reuse flows
    let flows = bond.build_schedule(curves, as_of)?;

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
    candidates.push((bond.maturity, bond.notional));

    // Solve YTM for each candidate and pick the smallest
    let mut best_price = 0.0;
    let mut best_yield = f64::INFINITY;
    for (exercise_date, redemption) in candidates {
        // Truncate flows to exercise and add redemption
        let mut ex_flows: Vec<(Date, Money)> = Vec::new();
        for &(d, a) in &flows {
            if d > as_of && d <= exercise_date {
                ex_flows.push((d, a));
            }
        }
        ex_flows.push((exercise_date, redemption));
        // Solve yield that matches target dirty price, then compute price from that yield
        let y = crate::instruments::bond::pricing::ytm_solver::solve_ytm(
            &ex_flows,
            as_of,
            dirty_price_target,
            crate::instruments::bond::pricing::ytm_solver::YtmPricingSpec {
                day_count: bond.dc,
                notional: bond.notional,
                coupon_rate: bond.coupon,
                compounding: YieldCompounding::Street,
                frequency: bond.freq,
            },
        )?;
        if y < best_yield {
            best_yield = y;
            best_price =
                price_from_ytm_compounded(bond, &ex_flows, as_of, y, YieldCompounding::Street)?;
        }
    }
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
    let disc = curves.get_discount_ref(bond.disc_id.clone())?;
    let base_date = disc.base_date();
    let mut pv = 0.0;
    for (d, a) in &flows {
        if *d <= as_of {
            continue;
        }
        let t = bond
            .dc
            .year_fraction(base_date, *d, DayCountCtx::default())
            .unwrap_or(0.0);
        let df = disc.df_on_date_curve(*d);
        let df_z = df * (-z * t).exp();
        pv += a.amount() * df_z;
    }
    Ok(pv)
}

/// Price from Option-Adjusted Spread using the short-rate tree pricer (expects bp input)
pub fn price_from_oas(
    bond: &Bond,
    curves: &MarketContext,
    as_of: Date,
    oas_bp: f64,
) -> finstack_core::Result<f64> {
    // Use the short-rate tree directly to price at a given OAS
    use crate::instruments::bond::pricing::tree_pricer::BondValuator;
    use crate::instruments::common::models::{
        short_rate_keys, ShortRateTree, ShortRateTreeConfig, StateVariables, TreeModel,
    };
    let time_to_maturity = bond
        .dc
        .year_fraction(as_of, bond.maturity, DayCountCtx::default())
        .unwrap_or(0.0);
    if time_to_maturity <= 0.0 {
        return Ok(0.0);
    }
    let discount_curve = curves.get_discount_ref(bond.disc_id.clone())?;
    let mut short_rate_tree = ShortRateTree::new(ShortRateTreeConfig::default());
    short_rate_tree.calibrate(discount_curve, time_to_maturity)?;
    let valuator = BondValuator::new(bond.clone(), curves, time_to_maturity, 100)?;
    let mut vars = StateVariables::new();
    vars.insert(short_rate_keys::OAS, oas_bp);
    let price = short_rate_tree.price(vars, time_to_maturity, curves, &valuator)?;
    Ok(price)
}

/// Price from a spread applied to an annuity approximation.
fn price_from_annuity_spread(
    bond: &Bond,
    curves: &MarketContext,
    as_of: Date,
    spread: f64,
) -> finstack_core::Result<f64> {
    let flows = bond.build_schedule(curves, as_of)?;
    let disc = curves.get_discount_ref(bond.disc_id.clone())?;
    let mut pv = 0.0;
    for (d, a) in &flows {
        if *d <= as_of {
            continue;
        }
        let df = disc.df_on_date_curve(*d);
        pv += a.amount() * df;
    }
    // As an approximation path, add the spread annuity contribution
    // Build a simple annual schedule
    let dates = super::schedule_helpers::build_annual_schedule(as_of, bond.maturity);
    let mut ann = 0.0;
    for w in dates.windows(2) {
        let (a, b) = (w[0], w[1]);
        let alpha = bond
            .dc
            .year_fraction(a, b, DayCountCtx::default())
            .unwrap_or(0.0);
        let p = disc.df_on_date_curve(b);
        ann += alpha * p;
    }
    let notional = bond.notional.amount();
    Ok(pv + notional * spread * ann)
}

/// Price from I-spread (approximate) by discount-ratio with annual schedule
pub fn price_from_i_spread(
    bond: &Bond,
    curves: &MarketContext,
    as_of: Date,
    i_spread: f64,
) -> finstack_core::Result<f64> {
    price_from_annuity_spread(bond, curves, as_of, i_spread)
}

/// Price from Asset Swap Spread (par/market agnostic) using annuity approximation
pub fn price_from_asw_spread(
    bond: &Bond,
    curves: &MarketContext,
    as_of: Date,
    asw_spread: f64,
) -> finstack_core::Result<f64> {
    price_from_annuity_spread(bond, curves, as_of, asw_spread)
}

/// Price from Discount Margin for FRNs by adding DM (decimal) to float margin and delegating to pricer
pub fn price_from_dm(
    bond: &Bond,
    curves: &MarketContext,
    as_of: Date,
    dm: f64,
) -> finstack_core::Result<f64> {
    if bond.float.is_none() {
        return Ok(bond.value(curves, as_of)?.amount());
    }
    let mut b = bond.clone();
    if let Some(ref mut fl) = b.float {
        fl.margin_bp += dm * 1e4;
    }
    Ok(b.value(curves, as_of)?.amount())
}

/// Returns the default schedule parameters used across accrual/pricers to avoid duplication.
#[inline]
pub fn default_schedule_params() -> (StubKind, BusinessDayConvention, Option<&'static str>) {
    (StubKind::None, BusinessDayConvention::Following, None)
}

/// Compute accrued interest between the last and next coupon dates.
///
/// If custom cashflows exist, uses Fixed/Stub coupon flows for accrual; otherwise,
/// uses generated schedule based on bond fields and linear accrual with the bond day count.
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
                let total_period = bond
                    .dc
                    .year_fraction(start_date, end_date, DayCountCtx::default())
                    .unwrap_or(0.0);
                let elapsed = bond
                    .dc
                    .year_fraction(start_date, as_of, DayCountCtx::default())
                    .unwrap_or(0.0)
                    .max(0.0);
                if total_period > 0.0 {
                    return Ok(coupon_amount.amount() * (elapsed / total_period));
                }
            }
        }
        return Ok(0.0);
    }

    // Fallback to canonical schedule using bond fields
    // Use instrument schedule conventions
    let sched = crate::cashflow::builder::build_dates(
        bond.issue,
        bond.maturity,
        bond.freq,
        bond.stub,
        bond.bdc,
        bond.calendar_id,
    );
    for window in sched.dates.windows(2) {
        let start_date = window[0];
        let end_date = window[1];
        // If ex-coupon is set, treat dates within ex-coupon window as zero accrual
        if let Some(ex_days) = bond.ex_coupon_days {
            let ex_date = end_date - Duration::days(ex_days as i64);
            if as_of >= ex_date && as_of < end_date {
                return Ok(0.0);
            }
        }
        if start_date <= as_of && as_of < end_date {
            let yf = bond
                .dc
                .year_fraction(start_date, end_date, DayCountCtx::default())
                .unwrap_or(0.0);
            let period_coupon = bond.notional.amount() * bond.coupon * yf;
            let elapsed = bond
                .dc
                .year_fraction(start_date, as_of, DayCountCtx::default())
                .unwrap_or(0.0)
                .max(0.0);
            if yf > 0.0 {
                return Ok(period_coupon * (elapsed / yf));
            }
        }
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
    if bond.float.is_none() || bond.custom_cashflows.is_some() {
        return compute_accrued_interest(bond, as_of);
    }

    // FRN path: approximate accrual using forward rate fixed at last reset
    let fl = bond.float.as_ref().unwrap();
    let fwd = curves.get_forward_ref(fl.fwd_id.as_str())?;

    // Build schedule with instrument conventions to locate current coupon window
    let sched = crate::cashflow::builder::build_dates(
        bond.issue,
        bond.maturity,
        bond.freq,
        bond.stub,
        bond.bdc,
        bond.calendar_id,
    );
    let dates = sched.dates;
    for w in dates.windows(2) {
        let start = w[0];
        let end = w[1];
        if start <= as_of && as_of < end {
            // Determine reset date and forward time
            let mut reset_date = start - Duration::days(fl.reset_lag_days as i64);
            if let Some(id) = bond.calendar_id {
                if let Some(cal) = calendar_by_id(id) {
                    reset_date = adjust(reset_date, bond.bdc, cal)?;
                }
            }
            let t_reset = fwd
                .day_count()
                .year_fraction(fwd.base_date(), reset_date, DayCountCtx::default())
                .unwrap_or(0.0);
            let yf_total = bond
                .dc
                .year_fraction(start, end, DayCountCtx::default())
                .unwrap_or(0.0);
            let yf_elapsed = bond
                .dc
                .year_fraction(start, as_of, DayCountCtx::default())
                .unwrap_or(0.0)
                .max(0.0);
            if yf_total <= 0.0 {
                return Ok(0.0);
            }
            let rate = fl.gearing * fwd.rate(t_reset) + fl.margin_bp * 1e-4;
            // Use current outstanding approximation as full notional for accrual
            let coupon_total = bond.notional.amount() * rate * yf_total;
            return Ok(coupon_total * (yf_elapsed / yf_total));
        }
    }
    Ok(0.0)
}
