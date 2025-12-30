//! Helper functions for cashflow emission.

use crate::cashflow::primitives::{CFKind, CashFlow};
use finstack_core::currency::Currency;
use finstack_core::dates::calendar::calendar_by_id;
use finstack_core::dates::HolidayCalendar;
use finstack_core::dates::{adjust, Date, DateExt};
use finstack_core::money::Money;

/// Add a PIK cashflow if the amount is nonzero.
///
/// Returns the PIK amount for outstanding balance tracking.
#[inline]
pub(in crate::cashflow::builder) fn add_pik_flow_if_nonzero(
    flows: &mut Vec<CashFlow>,
    date: Date,
    pik_amt: f64,
    ccy: Currency,
) -> f64 {
    if pik_amt > 0.0 {
        flows.push(CashFlow {
            date,
            reset_date: None,
            amount: Money::new(pik_amt, ccy),
            kind: CFKind::PIK,
            accrual_factor: 0.0,
            rate: None,
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
pub(in crate::cashflow::builder) fn compute_reset_date(
    accrual_start: Date,
    reset_lag_days: i32,
    bdc: finstack_core::dates::BusinessDayConvention,
    calendar_id: &Option<String>,
) -> finstack_core::Result<Date> {
    if reset_lag_days == 0 {
        if let Some(cal) = resolve_calendar(calendar_id.as_deref()) {
            return adjust(accrual_start, bdc, cal);
        }
        return Ok(accrual_start);
    }

    match resolve_calendar(calendar_id.as_deref()) {
        Some(cal) => {
            // Business-day subtraction avoids weekend/holiday traps where calendar-day subtraction
            // plus ModifiedFollowing could accidentally roll past the accrual start/end.
            let mut reset_date = accrual_start.add_business_days(-reset_lag_days, cal)?;
            reset_date = adjust(reset_date, bdc, cal)?;
            Ok(reset_date)
        }
        // Fallback: weekday-only business day arithmetic (Mon-Fri), ignores holidays.
        // This is strictly better than calendar-day shifting for market conventions like T-2.
        None => Ok(accrual_start.add_weekdays(-reset_lag_days)),
    }
}

/// Resolve calendar from optional ID string.
///
/// Centralized helper for looking up calendars by ID.
#[inline]
pub(in crate::cashflow::builder) fn resolve_calendar(
    calendar_id: Option<&str>,
) -> Option<&'static dyn HolidayCalendar> {
    calendar_id.and_then(calendar_by_id)
}
