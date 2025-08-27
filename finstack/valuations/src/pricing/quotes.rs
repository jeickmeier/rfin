#![allow(missing_docs)]

use finstack_core::prelude::*;
use finstack_core::F;

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
    let yf = dc.year_fraction(last_coupon, next_coupon).unwrap_or(0.0);
    let elapsed = dc.year_fraction(last_coupon, settle).unwrap_or(0.0);
    let period_coupon = notional * (coupon_rate * yf);
    period_coupon * (elapsed / yf)
}

#[inline]
pub fn dirty_from_clean(clean: F, accrued: F) -> F { clean + accrued }

#[inline]
pub fn clean_from_dirty(dirty: F, accrued: F) -> F { dirty - accrued }


