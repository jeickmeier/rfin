//! Helper functions for cashflow emission.

use crate::cashflow::primitives::{CashFlow, CFKind};
use finstack_core::currency::Currency;
use finstack_core::dates::{adjust, Date};
use finstack_core::dates::calendar::calendar_by_id;
use finstack_core::money::Money;
use time::Duration;

/// Add a PIK cashflow if the amount is nonzero.
///
/// Returns the PIK amount for outstanding balance tracking.
#[inline]
pub(super) fn add_pik_flow_if_nonzero(
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
/// Applies business day convention and calendar adjustment to the reset date
/// derived from the payment date minus reset lag days.
#[inline]
pub(super) fn compute_reset_date(
    payment_date: Date,
    reset_lag_days: i32,
    bdc: finstack_core::dates::BusinessDayConvention,
    calendar_id: &Option<String>,
) -> finstack_core::Result<Date> {
    let mut reset_date = payment_date - Duration::days(reset_lag_days as i64);
    if let Some(id) = calendar_id {
        if let Some(cal) = calendar_by_id(id) {
            reset_date = adjust(reset_date, bdc, cal)?;
        }
    }
    Ok(reset_date)
}

