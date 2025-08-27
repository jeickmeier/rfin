#![allow(missing_docs)]

use finstack_core::prelude::*;
use finstack_core::F;
use finstack_core::market_data::traits::Discount;

/// Year fraction between base and date using the given day-count.
#[inline]
pub fn year_fraction(base: Date, date: Date, dc: DayCount) -> F {
    // `DayCount::year_fraction` returns Result<F>; fall back to 0.0 only if equal dates.
    if date == base {
        return 0.0;
    }
    dc.year_fraction(base, date).unwrap_or(0.0)
}

/// Discount factor on a date given a discount curve and base date.
#[inline]
pub fn df_on(disc: &dyn Discount, base: Date, date: Date, dc: DayCount) -> F {
    let t = year_fraction(base, date, dc);
    disc.df(t)
}


