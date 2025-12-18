//! Date and business day utilities for interest rate swaps.

use finstack_core::dates::{Date, DateExt};
use finstack_core::dates::calendar::registry::CalendarRegistry;

/// Apply a payment-delay in business days using an optional holiday calendar.
///
/// Bloomberg/ISDA conventions define payment delay in **business days**, not just weekdays.
/// If a calendar is provided and found in the registry, we apply holiday-aware business day
/// addition; otherwise we fall back to weekday-only addition.
#[inline]
pub(crate) fn add_payment_delay(date: Date, delay_days: i32, calendar_id: Option<&str>) -> Date {
    if delay_days <= 0 {
        return date;
    }

    if let Some(id) = calendar_id {
        match CalendarRegistry::global().resolve_str(id) {
            Some(cal) => match date.add_business_days(delay_days, cal) {
                Ok(d) => return d,
                Err(e) => {
                    tracing::warn!(
                        calendar_id = id,
                        date = %date,
                        delay_days,
                        err = %e,
                        "Failed holiday-aware business-day addition for payment delay; falling back to weekday-only adjustment (Mon-Fri)"
                    );
                }
            },
            None => {
                tracing::warn!(
                    calendar_id = id,
                    date = %date,
                    delay_days,
                    "Payment-delay calendar not found; falling back to weekday-only adjustment (Mon-Fri)"
                );
            }
        };
    }

    // Fallback: weekday-only (Mon-Fri), ignores holidays.
    date.add_weekdays(delay_days)
}

