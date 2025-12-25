//! Credit event cashflow emission (defaults, prepayments, recoveries).

use crate::cashflow::primitives::{CFKind, CashFlow};
use finstack_core::currency::Currency;
use finstack_core::dates::calendar::registry::CalendarRegistry;
use finstack_core::dates::DateExt;
use finstack_core::dates::{adjust, Date};
use finstack_core::money::Money;
use tracing::warn;

use super::super::specs::DefaultEvent;

/// Emit default and recovery cashflows on a specific date.
///
/// For each default event on date `d`:
/// 1. Emit DefaultedNotional cashflow (reduces outstanding)
/// 2. Emit Recovery cashflow on future date (increases outstanding)
///
/// Net outstanding change = -defaulted_amount × (1 - recovery_rate)
///
/// # Arguments
///
/// * `d` - Current date to check for default events
/// * `default_events` - Slice of default event specifications
/// * `outstanding` - Mutable reference to outstanding notional balance
/// * `ccy` - Currency for cashflows
///
/// # Returns
///
/// Vector of cashflows (0, 1, or 2 per matching event)
///
/// # Examples
///
/// ```
/// use finstack_valuations::cashflow::builder::emit_default_on;
/// use finstack_valuations::cashflow::builder::specs::DefaultEvent;
/// use finstack_core::currency::Currency;
/// use finstack_core::dates::Date;
/// use time::Month;
///
/// let d = Date::from_calendar_date(2025, Month::March, 1).expect("valid date");
/// let event = DefaultEvent {
///     default_date: d,
///     defaulted_amount: 100_000.0,
///     recovery_rate: 0.40,
///     recovery_lag: 12,
///     recovery_bdc: None,
///     recovery_calendar_id: None,
/// };
/// let mut outstanding = 1_000_000.0;
/// let flows = emit_default_on(d, &[event], &mut outstanding, Currency::USD).expect("should succeed");
///
/// // Outstanding should be reduced by net loss (60% of defaulted amount)
/// assert_eq!(outstanding, 1_000_000.0 - 100_000.0 + 40_000.0); // 940K
/// assert_eq!(flows.len(), 2); // Default + Recovery cashflows
/// ```
pub fn emit_default_on(
    d: Date,
    default_events: &[DefaultEvent],
    outstanding: &mut f64,
    ccy: Currency,
) -> finstack_core::Result<Vec<CashFlow>> {
    let mut flows = Vec::new();

    for event in default_events.iter().filter(|e| e.default_date == d) {
        // Validate event parameters (recovery_rate in [0,1], defaulted_amount >= 0)
        event.validate()?;

        if event.defaulted_amount <= 0.0 {
            continue;
        }

        // Default cashflow
        flows.push(CashFlow {
            date: d,
            reset_date: None,
            amount: Money::new(event.defaulted_amount, ccy),
            kind: CFKind::DefaultedNotional,
            accrual_factor: 0.0,
            rate: None,
        });
        *outstanding -= event.defaulted_amount;

        // Recovery cashflow (on future date)
        let recovery_amt = event.defaulted_amount * event.recovery_rate;
        if recovery_amt > 0.0 {
            let base_recovery_date = d.add_months(event.recovery_lag as i32);

            // Apply optional business-day adjustment if both BDC and calendar are provided.
            let recovery_date = if let (Some(bdc), Some(ref cal_id)) =
                (event.recovery_bdc, &event.recovery_calendar_id)
            {
                // Resolve calendar by string code; warn and fall back to unadjusted date on failure.
                if let Some(cal) = CalendarRegistry::global().resolve_str(cal_id.as_str()) {
                    adjust(base_recovery_date, bdc, cal)?
                } else {
                    warn!(
                        calendar_id = %cal_id,
                        recovery_date = %base_recovery_date,
                        "Calendar not found for recovery date adjustment, using unadjusted date"
                    );
                    base_recovery_date
                }
            } else {
                base_recovery_date
            };
            flows.push(CashFlow {
                date: recovery_date,
                reset_date: None,
                amount: Money::new(recovery_amt, ccy),
                kind: CFKind::Recovery,
                accrual_factor: 0.0,
                rate: None,
            });
            *outstanding += recovery_amt;
        }
    }

    Ok(flows)
}

/// Emit prepayment cashflow on a specific date.
///
/// Reduces outstanding balance by prepayment amount.
/// Prepayments are unscheduled principal reductions, typically
/// driven by behavioral models (CPR/PSA for mortgages, etc.).
///
/// # Arguments
///
/// * `d` - Payment date
/// * `prepayment_amount` - Amount prepaid
/// * `outstanding` - Mutable reference to outstanding balance
/// * `ccy` - Currency
///
/// # Returns
///
/// Vector containing zero or one cashflow
///
/// # Examples
///
/// ```
/// use finstack_valuations::cashflow::builder::emit_prepayment_on;
/// use finstack_core::currency::Currency;
/// use finstack_core::dates::Date;
/// use time::Month;
///
/// let d = Date::from_calendar_date(2025, Month::March, 1).expect("valid date");
/// let mut outstanding = 1_000_000.0;
/// let flows = emit_prepayment_on(d, 50_000.0, &mut outstanding, Currency::USD);
///
/// assert_eq!(outstanding, 950_000.0);
/// assert_eq!(flows.len(), 1);
/// ```
pub fn emit_prepayment_on(
    d: Date,
    prepayment_amount: f64,
    outstanding: &mut f64,
    ccy: Currency,
) -> Vec<CashFlow> {
    if prepayment_amount <= 0.0 {
        return vec![];
    }

    let amount = prepayment_amount.min(*outstanding);
    if amount > 0.0 {
        *outstanding -= amount;
        vec![CashFlow {
            date: d,
            reset_date: None,
            amount: Money::new(amount, ccy),
            kind: CFKind::PrePayment,
            accrual_factor: 0.0,
            rate: None,
        }]
    } else {
        vec![]
    }
}
