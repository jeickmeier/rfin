#![allow(missing_docs)]

use finstack_core::prelude::*;
use finstack_core::F;
use finstack_core::market_data::term_structures::discount_curve::DiscountCurve;
use finstack_core::math::root_finding::brent;

/// Accrued interest for fixed coupon bonds.
pub fn accrued_interest(
    notional: Money,
    coupon_rate: F,
    last_coupon: Date,
    settle: Date,
    next_coupon: Date,
    dc: DayCount,
) -> Money {
    if settle <= last_coupon || settle >= next_coupon {
        return Money::new(0.0, notional.currency());
    }
    let yf = DiscountCurve::year_fraction(last_coupon, next_coupon, dc);
    let elapsed = DiscountCurve::year_fraction(last_coupon, settle, dc);
    let period_coupon = notional * (coupon_rate * yf);
    period_coupon * (elapsed / yf)
}

#[inline]
pub fn dirty_from_clean(clean: F, accrued: F) -> F { clean + accrued }

#[inline]
pub fn clean_from_dirty(dirty: F, accrued: F) -> F { dirty - accrued }

/// Discount a cashflow occurring at `date` from `as_of` using a flat yield `ytm`.
#[inline]
fn df_from_ytm(as_of: Date, date: Date, dc: DayCount, ytm: F) -> F {
    if date <= as_of { return 0.0; }
    let t = DiscountCurve::year_fraction(as_of, date, dc);
    if t <= 0.0 { return 0.0; }
    (1.0 + ytm).powf(-t)
}

/// Compute dirty price from a flat annual yield `ytm` using actual year fractions.
pub fn bond_dirty_from_ytm(
    notional: Money,
    coupon_rate: F,
    schedule: &[Date],
    dc: DayCount,
    as_of: Date,
    ytm: F,
) -> finstack_core::Result<Money> {
    let mut pv = Money::new(0.0, notional.currency());
    if schedule.is_empty() { return Ok(pv); }
    let mut prev = schedule[0];
    for &d in &schedule[1..] {
        let yf = DiscountCurve::year_fraction(prev, d, dc);
        let cpn = notional * (coupon_rate * yf);
        let df = df_from_ytm(as_of, d, dc, ytm);
        pv = (pv + (cpn * df))?;
        prev = d;
    }
    // Redemption at final date
    let df_mat = df_from_ytm(as_of, *schedule.last().unwrap(), dc, ytm);
    pv = (pv + (notional * df_mat))?;
    Ok(pv)
}

/// Same as `bond_dirty_from_ytm` but allows a custom redemption amount at the last date.
pub fn bond_dirty_from_ytm_with_redemption(
    notional: Money,
    coupon_rate: F,
    schedule: &[Date],
    dc: DayCount,
    as_of: Date,
    ytm: F,
    redemption: Money,
) -> finstack_core::Result<Money> {
    let mut pv = Money::new(0.0, notional.currency());
    if schedule.is_empty() { return Ok(pv); }
    let mut prev = schedule[0];
    for &d in &schedule[1..] {
        let yf = DiscountCurve::year_fraction(prev, d, dc);
        let cpn = notional * (coupon_rate * yf);
        let df = df_from_ytm(as_of, d, dc, ytm);
        pv = (pv + (cpn * df))?;
        prev = d;
    }
    let df_mat = df_from_ytm(as_of, *schedule.last().unwrap(), dc, ytm);
    pv = (pv + (redemption * df_mat))?;
    Ok(pv)
}

/// Solve YTM from dirty price for a truncated schedule with custom redemption amount.
pub fn bond_ytm_from_dirty_with_redemption(
    notional: Money,
    coupon_rate: F,
    schedule: &[Date],
    dc: DayCount,
    as_of: Date,
    dirty_price: Money,
    redemption: Money,
) -> F {
    let target = dirty_price.amount();
    let f = |y: f64| -> f64 {
        let pv = bond_dirty_from_ytm_with_redemption(notional, coupon_rate, schedule, dc, as_of, y, redemption)
            .map(|m| m.amount())
            .unwrap_or(0.0);
        pv - target
    };
    let mut a = -0.99;
    let mut b = 1.0;
    let mut root = brent(f, a, b, 1e-10, 128).unwrap_or(0.05);
    if !root.is_finite() {
        a = -0.99; b = 5.0;
        root = brent(f, a, b, 1e-10, 256).unwrap_or(0.05);
    }
    if !root.is_finite() { 0.05 } else { root }
}

/// Solve YTM from a dirty price using Brent's method on continuous time exponents (year fractions).
pub fn bond_ytm_from_dirty(
    notional: Money,
    coupon_rate: F,
    schedule: &[Date],
    dc: DayCount,
    as_of: Date,
    dirty_price: Money,
) -> F {
    // Objective: f(y) = PV_y(y) - dirty_price = 0
    let target = dirty_price.amount();
    let f = |y: f64| -> f64 {
        let pv = bond_dirty_from_ytm(notional, coupon_rate, schedule, dc, as_of, y).map(|m| m.amount()).unwrap_or(0.0);
        pv - target
    };
    // Try a bracket [ -0.99, 1.0 ] (~ -99% .. 100% yield) then widen if needed
    let mut a = -0.99;
    let mut b = 1.0;
    let mut root = brent(f, a, b, 1e-10, 128).unwrap_or(0.05);
    // If solver failed to converge due to no sign change, try wider bounds
    if !root.is_finite() {
        a = -0.99; b = 5.0;
        root = brent(f, a, b, 1e-10, 256).unwrap_or(0.05);
    }
    // Ensure result is finite
    if !root.is_finite() { 0.05 } else { root }
}

/// Macaulay and Modified duration computed from a flat yield and actual year fractions.
pub fn bond_duration_mac_mod(
    notional: Money,
    coupon_rate: F,
    schedule: &[Date],
    dc: DayCount,
    as_of: Date,
    ytm: F,
) -> (F, F) {
    let price = bond_dirty_from_ytm(notional, coupon_rate, schedule, dc, as_of, ytm).map(|m| m.amount()).unwrap_or(0.0);
    if price == 0.0 { return (0.0, 0.0); }
    let mut num = 0.0;
    let mut prev = schedule[0];
    for &d in &schedule[1..] {
        let yf = DiscountCurve::year_fraction(prev, d, dc);
        let cpn_amt = (notional * (coupon_rate * yf)).amount();
        let t = DiscountCurve::year_fraction(as_of, d, dc).max(0.0);
        let df = (1.0 + ytm).powf(-t);
        num += t * cpn_amt * df;
        prev = d;
    }
    // Redemption
    let t_mat = DiscountCurve::year_fraction(as_of, *schedule.last().unwrap(), dc).max(0.0);
    let df_mat = (1.0 + ytm).powf(-t_mat);
    num += t_mat * notional.amount() * df_mat;

    let d_mac = num / price;
    let d_mod = d_mac / (1.0 + ytm);
    (d_mac, d_mod)
}

/// Numerical convexity using central difference around `ytm` with bump `dy`.
pub fn bond_convexity_numeric(
    notional: Money,
    coupon_rate: F,
    schedule: &[Date],
    dc: DayCount,
    as_of: Date,
    ytm: F,
    dy: F,
) -> F {
    let p0 = bond_dirty_from_ytm(notional, coupon_rate, schedule, dc, as_of, ytm).map(|m| m.amount()).unwrap_or(0.0);
    let p_up = bond_dirty_from_ytm(notional, coupon_rate, schedule, dc, as_of, ytm + dy).map(|m| m.amount()).unwrap_or(0.0);
    let p_dn = bond_dirty_from_ytm(notional, coupon_rate, schedule, dc, as_of, ytm - dy).map(|m| m.amount()).unwrap_or(0.0);
    if p0 == 0.0 || dy == 0.0 { return 0.0; }
    (p_up + p_dn - 2.0 * p0) / (p0 * dy * dy)
}

/// Numerical CS01 via z-spread bump: price change per 1bp change in yield.
pub fn bond_cs01_zspread(
    notional: Money,
    coupon_rate: F,
    schedule: &[Date],
    dc: DayCount,
    as_of: Date,
    ytm: F,
    bp: F,
) -> F {
    let p_up = bond_dirty_from_ytm(notional, coupon_rate, schedule, dc, as_of, ytm + bp).map(|m| m.amount()).unwrap_or(0.0);
    let p_dn = bond_dirty_from_ytm(notional, coupon_rate, schedule, dc, as_of, ytm - bp).map(|m| m.amount()).unwrap_or(0.0);
    (p_dn - p_up) / (2.0 * bp)
}


