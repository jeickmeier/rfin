#![allow(missing_docs)]

use crate::instruments::bond::Bond;
use crate::metrics::MetricContext;
use crate::traits::CashflowProvider;
use finstack_core::prelude::*;

/// Return cached cashflows from context or build and cache them.
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

/// Price a stream of flows using a flat yield compounded discretely to time t via (1+y)^t.
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


