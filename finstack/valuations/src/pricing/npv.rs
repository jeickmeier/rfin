#![allow(missing_docs)]

use finstack_core::prelude::*;
use finstack_core::market_data::traits::Discount;
use finstack_core::market_data::term_structures::discount_curve::DiscountCurve;


/// Compute NPV of dated `Money` flows using a `Discount` curve and `DayCount`.
pub fn npv(
    disc: &dyn Discount,
    base: Date,
    dc: DayCount,
    flows: &[(Date, Money)],
) -> finstack_core::Result<Money> {
    if flows.is_empty() {
        return Ok(Money::new(0.0, Currency::USD));
    }
    let ccy = flows[0].1.currency();
    let mut total = Money::new(0.0, ccy);
    for (d, amt) in flows {
        let df = DiscountCurve::df_on(disc, base, *d, dc);
        // Multiplying Money by scalar returns Money
        let disc_amt = *amt * df;
        total = (total + disc_amt)?;
    }
    Ok(total)
}
