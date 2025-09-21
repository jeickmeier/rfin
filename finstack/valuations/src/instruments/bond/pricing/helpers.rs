//! Bond pricing helpers (moved from bond/helpers.rs)

use super::super::types::Bond;
use finstack_core::dates::{BusinessDayConvention, DayCountCtx, StubKind};
use finstack_core::market_data::MarketContext;
use finstack_core::dates::calendar::calendar_by_id;
use finstack_core::dates::adjust;
use time::Duration;
use finstack_core::F;


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
) -> finstack_core::Result<finstack_core::F> {
    match freq {
        finstack_core::dates::Frequency::Months(m) => {
            if m == 0 {
                return Err(finstack_core::error::InputError::Invalid.into());
            }
            Ok(12.0 / (m as finstack_core::F))
        }
        finstack_core::dates::Frequency::Days(d) => {
            if d == 0 {
                return Err(finstack_core::error::InputError::Invalid.into());
            }
            Ok(365.0 / (d as finstack_core::F))
        }
        _ => Err(finstack_core::error::InputError::Invalid.into()),
    }
}

#[inline]
pub fn df_from_yield(
    ytm: finstack_core::F,
    t: finstack_core::F,
    comp: YieldCompounding,
    bond_freq: finstack_core::dates::Frequency,
) -> finstack_core::Result<finstack_core::F> {
    if t <= 0.0 {
        return Ok(1.0);
    }
    Ok(match comp {
        YieldCompounding::Simple => 1.0 / (1.0 + ytm * t),
        YieldCompounding::Annual => (1.0 + ytm).powf(-t),
        YieldCompounding::Periodic(m) => {
            let m = m as finstack_core::F;
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
    ytm: finstack_core::F,
) -> finstack_core::Result<finstack_core::F> {
    price_from_ytm_compounded(bond, flows, as_of, ytm, YieldCompounding::Street)
}

/// Price from yield using explicit day count and frequency (no `Bond` borrow required).
#[inline]
pub fn price_from_ytm_compounded_params(
    day_count: finstack_core::dates::DayCount,
    freq: finstack_core::dates::Frequency,
    flows: &[(finstack_core::dates::Date, finstack_core::money::Money)],
    as_of: finstack_core::dates::Date,
    ytm: finstack_core::F,
    comp: YieldCompounding,
) -> finstack_core::Result<finstack_core::F> {
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
    ytm: finstack_core::F,
    comp: YieldCompounding,
) -> finstack_core::Result<finstack_core::F> {
    price_from_ytm_compounded_params(bond.dc, bond.freq, flows, as_of, ytm, comp)
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
) -> finstack_core::Result<F> {
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
) -> finstack_core::Result<F> {
    // If fixed or custom flows exist, fall back to standard helper and return
    if bond.float.is_none() || bond.custom_cashflows.is_some() {
        return compute_accrued_interest(bond, as_of);
    }

    // FRN path: approximate accrual using forward rate fixed at last reset
    let fl = bond.float.as_ref().unwrap();
    let fwd = curves.get_ref::<finstack_core::market_data::term_structures::forward_curve::ForwardCurve>(fl.fwd_id.as_str())?;

    // Build schedule with instrument conventions to locate current coupon window
    let sched = crate::cashflow::builder::build_dates(bond.issue, bond.maturity, bond.freq, bond.stub, bond.bdc, bond.calendar_id);
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
