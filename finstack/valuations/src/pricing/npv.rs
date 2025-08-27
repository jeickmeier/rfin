#![allow(missing_docs)]

use finstack_core::prelude::*;
use finstack_core::F;
use finstack_core::market_data::traits::Discount;

use super::df::df_on;

/// Compute NPV of dated `Money` flows using a `Discount` curve and `DayCount`.
pub fn npv(
    disc: &dyn Discount,
    base: Date,
    dc: DayCount,
    flows: &[(Date, Money)],
) -> Money {
    if flows.is_empty() {
        return Money::new(0.0, Currency::USD);
    }
    let ccy = flows[0].1.currency();
    let mut total = Money::new(0.0, ccy);
    for (d, amt) in flows {
        let df = df_on(disc, base, *d, dc);
        // Multiplying Money by scalar returns Money
        let disc_amt = *amt * df;
        total = (total + disc_amt).expect("currency mismatch");
    }
    total
}

/// Helper: present value of a level coupon schedule defined by (dates, yfs) and a fixed rate.
pub fn pv_fixed_leg(
    disc: &dyn Discount,
    base: Date,
    dc: DayCount,
    notional: Money,
    coupon_rate: F,
    schedule: &[Date],
) -> Money {
    let mut pv = Money::new(0.0, notional.currency());
    let mut prev = schedule[0];
    for &d in &schedule[1..] {
        let yf = dc.year_fraction(prev, d).unwrap_or(0.0);
        let cash = notional * (coupon_rate * yf);
        let df = df_on(disc, base, d, dc);
        let disc_cash = cash * df;
        pv = (pv + disc_cash).expect("currency mismatch");
        prev = d;
    }
    pv
}


