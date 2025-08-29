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
/// # Example
/// ```rust
/// use finstack_valuations::instruments::fixed_income::bond::helpers::flows_from_context_or_build;
/// use finstack_valuations::metrics::traits::MetricContext;
/// use finstack_valuations::instruments::fixed_income::bond::Bond;
/// use finstack_valuations::instruments::Instrument;
/// use std::sync::Arc;
/// use finstack_core::currency::Currency;
/// use finstack_core::money::Money;
/// use finstack_core::dates::{Date, Frequency, DayCount};
/// use time::Month;
/// 
/// // Create a simple bond for the example
/// let bond = Bond {
///     id: "BOND001".to_string(),
///     notional: Money::new(1000.0, Currency::USD),
///     coupon: 0.05,
///     freq: Frequency::semi_annual(),
///     dc: DayCount::Act365F,
///     issue: Date::from_calendar_date(2025, Month::January, 1).unwrap(),
///     maturity: Date::from_calendar_date(2026, Month::January, 1).unwrap(),
///     disc_id: "USD-OIS",
///     quoted_clean: None,
///     call_put: None,
///     amortization: None,
///     custom_cashflows: None,
///     attributes: finstack_valuations::traits::Attributes::new(),
/// };
/// 
/// let mut context = MetricContext::new(
///     Arc::new(Instrument::Bond(bond.clone())),
///     Arc::new(finstack_core::market_data::multicurve::CurveSet::new()),
///     Date::from_calendar_date(2025, Month::January, 1).unwrap(),
///     Money::new(1000.0, Currency::USD)
/// );
/// 
/// // Note: This would require proper market data to work
/// // let flows = flows_from_context_or_build(&mut context, &bond).unwrap();
/// // assert!(!flows.is_empty());
/// ```
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
/// # Example
/// ```rust
/// use finstack_valuations::instruments::fixed_income::bond::helpers::price_from_ytm;
/// use finstack_valuations::instruments::fixed_income::bond::Bond;
/// use finstack_core::dates::{Date, Frequency, DayCount};
/// use finstack_core::money::Money;
/// use finstack_core::currency::Currency;
/// use time::Month;
/// 
/// // Create a simple bond for the example
/// let bond = Bond {
///     id: "BOND001".to_string(),
///     notional: Money::new(1000.0, Currency::USD),
///     coupon: 0.05,
///     freq: Frequency::semi_annual(),
///     dc: DayCount::Act365F,
///     issue: Date::from_calendar_date(2025, Month::January, 1).unwrap(),
///     maturity: Date::from_calendar_date(2026, Month::January, 1).unwrap(),
///     disc_id: "USD-OIS",
///     quoted_clean: None,
///     call_put: None,
///     amortization: None,
///     custom_cashflows: None,
///     attributes: finstack_valuations::traits::Attributes::new(),
/// };
/// 
/// let flows = vec![
///     (Date::from_calendar_date(2025, Month::June, 15).unwrap(), 
///      Money::new(50.0, Currency::USD)),
///     (Date::from_calendar_date(2025, Month::December, 15).unwrap(), 
///      Money::new(1050.0, Currency::USD))
/// ];
/// let as_of = Date::from_calendar_date(2025, Month::January, 1).unwrap();
/// let ytm = 0.05; // 5% yield
/// 
/// let pv = price_from_ytm(&bond, &flows, as_of, ytm).unwrap();
/// assert!(pv > 0.0);
/// ```
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


