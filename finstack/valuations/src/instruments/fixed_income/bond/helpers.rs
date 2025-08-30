//! Bond-specific helper functions for cashflow management and pricing.
//! 
//! Provides utilities for retrieving cached cashflows and pricing bonds
//! using yield-to-maturity calculations with proper day count conventions.

use super::Bond;
use crate::metrics::MetricContext;
use crate::traits::CashflowProvider;
use finstack_core::prelude::*;

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
    use finstack_core::market_data::term_structures::discount_curve::DiscountCurve;
    let mut pv = 0.0;
    for &(date, amount) in flows {
        if date <= as_of { continue; }
        let t = DiscountCurve::year_fraction(as_of, date, bond.dc);
        if t > 0.0 {
            let df = (1.0 + ytm).powf(-t);
            pv += amount.amount() * df;
        }
    }
    Ok(pv)
}


