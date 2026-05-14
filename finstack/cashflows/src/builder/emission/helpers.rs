//! Helper functions for cashflow emission.

use crate::primitives::{CFKind, CashFlow};
use finstack_core::currency::Currency;
use finstack_core::dates::{adjust, Date, DateExt, HolidayCalendar};
use finstack_core::money::Money;

/// Add a PIK cashflow if the amount is nonzero.
///
/// Returns the PIK amount for outstanding balance tracking.
#[inline]
pub(in crate::builder) fn add_pik_flow_if_nonzero(
    flows: &mut Vec<CashFlow>,
    date: Date,
    pik_amt: f64,
    ccy: Currency,
    rate: Option<f64>,
    accrual_factor: f64,
) -> f64 {
    if pik_amt > 0.0 {
        flows.push(CashFlow {
            date,
            reset_date: None,
            amount: Money::new(pik_amt, ccy),
            kind: CFKind::PIK,
            accrual_factor,
            rate,
        });
        pik_amt
    } else {
        0.0
    }
}

/// Compute reset date with calendar adjustment.
///
/// Market standard: reset dates are computed as `accrual_start - reset_lag_days`
/// **business days** using the fixing calendar (or accrual calendar), then adjusted
/// to a business day using the specified business-day convention.
#[inline]
pub(in crate::builder) fn compute_reset_date(
    accrual_start: Date,
    reset_lag_days: i32,
    bdc: finstack_core::dates::BusinessDayConvention,
    cal: &dyn HolidayCalendar,
) -> finstack_core::Result<Date> {
    if reset_lag_days == 0 {
        return adjust(accrual_start, bdc, cal);
    }

    // Business-day subtraction avoids weekend/holiday traps where calendar-day subtraction
    // plus ModifiedFollowing could accidentally roll past the accrual start/end.
    let mut reset_date = accrual_start.add_business_days(-reset_lag_days, cal)?;
    reset_date = adjust(reset_date, bdc, cal)?;
    Ok(reset_date)
}
