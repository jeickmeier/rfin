//! Bond-specific helper functions for cashflow management and pricing.
//!
//! Provides utilities for retrieving cached cashflows and pricing bonds
//! using yield-to-maturity calculations with proper day count conventions.

use super::Bond;
use crate::metrics::MetricContext;
use crate::traits::CashflowProvider;
use finstack_core::prelude::*;

/// Yield compounding convention for discounting cashflows from a flat yield.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum YieldCompounding {
    /// Simple interest: df = 1 / (1 + y * t)
    Simple,
    /// Annual discrete compounding: df = (1 + y)^(-t)
    Annual,
    /// Periodic compounding m times per year: df = (1 + y/m)^(-m*t)
    Periodic(u32),
    /// Continuous compounding: df = exp(-y * t)
    Continuous,
    /// Street convention: periodic compounding with the instrument's coupon frequency
    Street,
}

/// Map frequency to periods per year.
///
/// In fixed-income, coupon frequencies are represented using only `Months(m)`
/// (e.g., 3 for quarterly, 6 for semi-annual) or `Days(d)` (e.g., 7 for weekly).
/// Any other `Frequency` variants are unsupported here and will cause a panic.
/// Zero values are invalid and will also cause a panic.
#[inline]
pub fn periods_per_year(freq: finstack_core::dates::Frequency) -> finstack_core::F {
    match freq {
        finstack_core::dates::Frequency::Months(m) => {
            assert!(m > 0, "Frequency::Months(0) is invalid for bond helpers");
            12.0 / (m as finstack_core::F)
        }
        finstack_core::dates::Frequency::Days(d) => {
            assert!(d > 0, "Frequency::Days(0) is invalid for bond helpers");
            365.0 / (d as finstack_core::F)
        }
        _ => panic!(
            "Unsupported Frequency variant in bond helpers: only Months(_) or Days(_) are allowed"
        ),
    }
}

/// Discount factor from a flat yield under a compounding convention.
#[inline]
pub fn df_from_yield(
    ytm: finstack_core::F,
    t: finstack_core::F,
    comp: YieldCompounding,
    bond_freq: finstack_core::dates::Frequency,
) -> finstack_core::F {
    if t <= 0.0 {
        return 1.0;
    }
    match comp {
        YieldCompounding::Simple => 1.0 / (1.0 + ytm * t),
        YieldCompounding::Annual => (1.0 + ytm).powf(-t),
        YieldCompounding::Periodic(m) => {
            let m = m as finstack_core::F;
            (1.0 + ytm / m).powf(-m * t)
        }
        YieldCompounding::Continuous => (-ytm * t).exp(),
        YieldCompounding::Street => {
            let m = periods_per_year(bond_freq).max(1.0);
            (1.0 + ytm / m).powf(-m * t)
        }
    }
}

/// Discount factor and its derivative with respect to yield under a compounding convention.
#[inline]
pub fn df_and_derivative_from_yield(
    ytm: finstack_core::F,
    t: finstack_core::F,
    comp: YieldCompounding,
    bond_freq: finstack_core::dates::Frequency,
) -> (finstack_core::F, finstack_core::F) {
    let df = df_from_yield(ytm, t, comp, bond_freq);
    if t <= 0.0 {
        return (df, 0.0);
    }
    let ddf_dy = match comp {
        YieldCompounding::Simple => {
            // df = 1/(1+y*t) => ddf/dy = -t / (1+y*t)^2
            let denom = 1.0 + ytm * t;
            -t / (denom * denom)
        }
        YieldCompounding::Annual => {
            // df = (1+y)^(-t) => ddf/dy = -t * (1+y)^(-t-1) = -t * df / (1+y)
            -t * df / (1.0 + ytm)
        }
        YieldCompounding::Periodic(m) => {
            // df = (1+y/m)^(-m*t) => ddf/dy = -t * (1+y/m)^(-m*t-1) = -t * df / (1+y/m)
            let m = m as finstack_core::F;
            -t * df / (1.0 + ytm / m)
        }
        YieldCompounding::Continuous => {
            // df = exp(-y*t) => ddf/dy = -t * exp(-y*t) = -t * df
            -t * df
        }
        YieldCompounding::Street => {
            let m = periods_per_year(bond_freq).max(1.0);
            -t * df / (1.0 + ytm / m)
        }
    };
    (df, ddf_dy)
}

/// Retrieves cached cashflows from context or builds and caches them.
///
/// This function optimizes performance by avoiding repeated cashflow
/// generation when the same bond is priced multiple times. It first
/// checks for existing cached flows, then builds new ones if needed.
///
/// # Arguments
/// * `context` - Metric context containing cached data and market curves
/// * `bond` - Bond instrument to generate cashflows for
///
/// # Returns
/// Vector of (date, money) tuples representing the bond's cashflow schedule
///
/// See unit tests and `examples/` for usage.
pub fn flows_from_context_or_build(
    context: &mut MetricContext,
    bond: &Bond,
) -> finstack_core::Result<Vec<(Date, Money)>> {
    if let Some(flows) = &context.cashflows {
        return Ok(flows.clone());
    }
    let flows = bond.build_schedule(&context.curves, context.as_of)?;
    context.cashflows = Some(flows.clone());
    context.discount_curve_id = Some(bond.disc_id);
    context.day_count = Some(bond.dc);
    Ok(flows)
}

/// Prices a stream of cashflows using a flat yield compounded discretely.
///
/// Calculates present value using the formula PV = Σ(CF_t / (1+y)^t) where
/// t is the year fraction from the valuation date. Only future cashflows
/// are included in the calculation.
///
/// # Arguments
/// * `bond` - Bond instrument providing day count convention and discount curve ID
/// * `flows` - Vector of (date, money) tuples representing cashflows
/// * `as_of` - Valuation date for present value calculation
/// * `ytm` - Yield to maturity as a decimal (e.g., 0.05 for 5%)
///
/// # Returns
/// Present value of the cashflow stream
///
/// See unit tests and `examples/` for usage.
pub fn price_from_ytm(
    bond: &Bond,
    flows: &[(Date, Money)],
    as_of: Date,
    ytm: finstack_core::F,
) -> finstack_core::Result<finstack_core::F> {
    price_from_ytm_compounded(bond, flows, as_of, ytm, YieldCompounding::Street)
}

/// Prices cashflows using a flat yield and explicit compounding convention.
pub fn price_from_ytm_compounded(
    bond: &Bond,
    flows: &[(Date, Money)],
    as_of: Date,
    ytm: finstack_core::F,
    comp: YieldCompounding,
) -> finstack_core::Result<finstack_core::F> {
    use finstack_core::market_data::term_structures::discount_curve::DiscountCurve;
    let mut pv = 0.0;
    for &(date, amount) in flows {
        if date <= as_of {
            continue;
        }
        let t = DiscountCurve::year_fraction(as_of, date, bond.dc);
        if t > 0.0 {
            let df = df_from_yield(ytm, t, comp, bond.freq);
            pv += amount.amount() * df;
        }
    }
    Ok(pv)
}
