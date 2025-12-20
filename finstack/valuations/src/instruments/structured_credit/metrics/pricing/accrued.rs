//! Accrued interest calculator for structured credit instruments.

use crate::metrics::{MetricCalculator, MetricContext, MetricId};
use finstack_core::dates::{Date, DayCount, DayCountCtx};
use finstack_core::money::Money;
use finstack_core::Result;

/// Calculates accrued interest for structured credit instruments.
///
/// Accrued interest is the pro-rata interest that has accrued since the last
/// payment date. For structured credit, this is calculated per tranche based on:
/// - Days elapsed since last payment
/// - Days in the current period
/// - Current coupon rate (which may be floating)
///
/// # Formula
///
/// Accrued = (Days Elapsed / Days in Period) × Coupon Rate × Notional
///
pub struct AccruedCalculator;

impl MetricCalculator for AccruedCalculator {
    fn calculate(&self, context: &mut MetricContext) -> Result<f64> {
        // For structured credit, we calculate accrued based on the payment schedule
        // This is a simplified implementation - in practice, you'd need to:
        // 1. Find the last payment date before as_of
        // 2. Find the next payment date after as_of
        // 3. Get the effective coupon rate for the period
        // 4. Calculate pro-rata accrual

        // Get cached cashflows to determine payment schedule
        let flows = context.cashflows.as_ref().ok_or_else(|| {
            finstack_core::Error::from(finstack_core::error::InputError::NotFound {
                id: "context.cashflows".to_string(),
            })
        })?;

        // If there are no payments, or we are before the first payment or after the final one,
        // there is no accrued interest to compute.
        if let (Some((first_date, _)), Some((last_date, _))) = (flows.first(), flows.last()) {
            if context.as_of < *first_date || context.as_of >= *last_date {
                return Ok(0.0);
            }
        } else {
            return Ok(0.0);
        }

        // Find surrounding payment dates
        let (last_payment, next_payment) = find_surrounding_dates(flows, context.as_of)?;

        // Calculate day count fraction using context day count if available
        let day_count = context.day_count.unwrap_or(DayCount::Act360);
        let accrual_fraction =
            day_count.year_fraction(last_payment, context.as_of, DayCountCtx::default())?;
        let period_fraction =
            day_count.year_fraction(last_payment, next_payment, DayCountCtx::default())?;

        if period_fraction == 0.0 {
            return Ok(0.0);
        }

        // Find the interest payment for this period
        let period_interest = flows
            .iter()
            .find(|(d, _)| *d == next_payment)
            .map(|(_, m)| m.amount())
            .unwrap_or(0.0);

        // Pro-rate the interest payment
        let accrued = period_interest * (accrual_fraction / period_fraction);

        Ok(accrued)
    }

    fn dependencies(&self) -> &[MetricId] {
        &[] // No dependencies - uses cashflows from context
    }
}

/// Helper to find the payment dates surrounding as_of date.
fn find_surrounding_dates(flows: &[(Date, Money)], as_of: Date) -> Result<(Date, Date)> {
    // Find last payment before or on as_of
    let last = flows
        .iter()
        .filter(|(d, _)| *d <= as_of)
        .map(|(d, _)| *d)
        .max();

    // Find next payment after as_of
    let next = flows
        .iter()
        .filter(|(d, _)| *d > as_of)
        .map(|(d, _)| *d)
        .min();

    match (last, next) {
        (Some(l), Some(n)) => Ok((l, n)),
        _ => Err(finstack_core::Error::from(
            finstack_core::error::InputError::NotFound {
                id: "accrual_period".to_string(),
            },
        )),
    }
}
